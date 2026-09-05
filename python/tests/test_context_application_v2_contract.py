from __future__ import annotations

import copy
import json
import unittest
from dataclasses import replace
from pathlib import Path

import jsonschema
from mtgml.authority import (
    ACCEPTANCE_CHECKLIST_V2,
    ACCEPTANCE_EVENT_SCHEMA_V3,
    ACCEPTANCE_SUBJECT_INPUT_SCHEMA_V3,
    CANONICAL_CBOR_ID,
    CONTEXT_APPLICATION_INPUT_SCHEMA_V2,
    CONTEXT_APPLICATION_RECORD_INPUT_SCHEMA_V2,
    CONTEXT_AUTHORITY_SCHEMA_V2,
    CONTEXT_AUTHORITY_SOURCE_ROLES_V2,
    CONTEXT_SUPERSESSION_INPUT_SCHEMA_V2,
    CONTEXT_SUPERSESSION_RECORD_INPUT_SCHEMA_V2,
    DIGEST_ENVELOPE_ID,
    REVIEWER_ROSTER_SCHEMA_V1,
    SHA256_ID,
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKindV3,
    AcceptanceSubjectPayloadV3,
    ApplicationHostBindingV2,
    AuthorityContractError,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationAuthorityV2,
    ContextApplicationMemberV2,
    ContextApplicationV2InputV1,
    ContextApplicationV2Record,
    ContextApplicationV2SupersessionInputV2,
    ContextApplicationV2SupersessionRecord,
    ContextAuthoritySourceBindingV2,
    ContextBridgeRelationV2,
    ContextMemberBridgeAttestationV2,
    ContextSlotBridgeAttestationV2,
    DigestReferenceV1,
    EvidenceRefV1,
    ReviewAcceptanceEventInputV3,
    ReviewAcceptanceEventLeafV3,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewEventRefV3,
    ReviewMode,
    SupersessionReason,
    TemporalSlotAttestationV2,
    canonical_identity_input,
    compute_authority_identity,
)
from mtgml.persistence import decode_canonical, encode_canonical

ROOT = Path(__file__).resolve().parents[2]
ZERO = bytes(32)


def digest_reference() -> DigestReferenceV1:
    return DigestReferenceV1(
        envelope_id=DIGEST_ENVELOPE_ID,
        algorithm_id=SHA256_ID,
        semantic_domain="manafold.m2.5.c.candidate-identity.v1",
        payload_codec_id=CANONICAL_CBOR_ID,
        input_schema_id="manafold.m2.5.c.candidate-identity-input.v1",
        digest_bytes=ZERO,
    )


def subject_digest_reference() -> DigestReferenceV1:
    return DigestReferenceV1(
        envelope_id=DIGEST_ENVELOPE_ID,
        algorithm_id=SHA256_ID,
        semantic_domain=ACCEPTANCE_SUBJECT_INPUT_SCHEMA_V3.replace("-input", ""),
        payload_codec_id=CANONICAL_CBOR_ID,
        input_schema_id=ACCEPTANCE_SUBJECT_INPUT_SCHEMA_V3,
        digest_bytes=ZERO,
    )


def evidence() -> EvidenceRefV1:
    return EvidenceRefV1(
        authority_kind="model",
        path="a",
        locator=("whole_artifact", None),
        raw_sha256=ZERO,
    )


def bridge() -> ContextMemberBridgeAttestationV2:
    context = tuple(
        ContextSlotBridgeAttestationV2(
            slot_name=name,
            source_value="not_applicable",
            reviewed_value="not_applicable",
            relation=ContextBridgeRelationV2.EXACT_MATCH,
            evidence_refs=(evidence(),),
            rationale="x",
        )
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
    )
    temporal = tuple(
        TemporalSlotAttestationV2(
            slot_name=name,
            reviewed_value="not_applicable",
            evidence_refs=(evidence(),),
            rationale="x",
        )
        for name in (
            "trigger_order",
            "dependency_order",
            "duration",
            "replacement_order",
        )
    )
    return ContextMemberBridgeAttestationV2(context=context, temporal=temporal)


def member() -> ContextApplicationMemberV2:
    return ContextApplicationMemberV2(
        candidate_id="c",
        candidate_identity_digest_reference=digest_reference(),
        source_instance_id="s",
        candidate_universe_binding=[
            "u",
            "manafold.m2.5.c.interaction-candidate-universe.v2",
            ZERO,
        ],
        context_binding_v1=[
            "binary",
            "symmetric",
            [[0, "ordered_participant", "card", "draw"]],
            "same_host",
        ],
        precondition_attestations_v1=[],
        member_evidence_refs=(evidence(),),
        context_member_bridge_attestation_v2=bridge(),
    )


class ContextApplicationV2ContractTests(unittest.TestCase):
    def test_registry_contains_exact_six_new_identity_families(self) -> None:
        expected = {
            AuthorityIdentityKind.CONTEXT_APPLICATION_V2: (
                "cpa.v2/",
                "manafold.m2.5.c.context-application.v2",
                CONTEXT_APPLICATION_INPUT_SCHEMA_V2,
                3,
            ),
            AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2: (
                "cpar.v2/",
                "manafold.m2.5.c.context-application-record.v2",
                CONTEXT_APPLICATION_RECORD_INPUT_SCHEMA_V2,
                3,
            ),
            AuthorityIdentityKind.CONTEXT_SUPERSESSION_V2: (
                "cps.v2/",
                "manafold.m2.5.c.context-application-supersession.v2",
                CONTEXT_SUPERSESSION_INPUT_SCHEMA_V2,
                7,
            ),
            AuthorityIdentityKind.CONTEXT_SUPERSESSION_RECORD_V2: (
                "cpsr.v2/",
                "manafold.m2.5.c.context-application-supersession-record.v2",
                CONTEXT_SUPERSESSION_RECORD_INPUT_SCHEMA_V2,
                3,
            ),
            AuthorityIdentityKind.ACCEPTANCE_SUBJECT_V3: (
                "asp.v3/",
                "manafold.m2.5.c.acceptance-subject-payload.v3",
                ACCEPTANCE_SUBJECT_INPUT_SCHEMA_V3,
                3,
            ),
            AuthorityIdentityKind.REVIEW_ACCEPTANCE_EVENT_V3: (
                "ae.v3/",
                "manafold.m2.5.c.review-acceptance-event.v3",
                "manafold.m2.5.c.review-acceptance-event-input.v3",
                10,
            ),
        }
        for kind, (prefix, domain, schema, _arity) in expected.items():
            with self.subTest(kind=kind):
                identity = AuthorityIdentityV1(kind, ZERO)
                self.assertEqual(identity.prefix, prefix)
                self.assertEqual(identity.semantic_domain, domain)
                self.assertEqual(identity.input_schema_id, schema)

    def test_digest_reference_and_member_use_fixed_cbor_arrays(self) -> None:
        candidate_ref = digest_reference()
        self.assertEqual(
            candidate_ref.to_cbor(),
            [
                DIGEST_ENVELOPE_ID,
                SHA256_ID,
                "manafold.m2.5.c.candidate-identity.v1",
                CANONICAL_CBOR_ID,
                "manafold.m2.5.c.candidate-identity-input.v1",
                ZERO,
            ],
        )
        observed_member = member().to_cbor()
        self.assertEqual(len(observed_member), 8)
        self.assertEqual(observed_member[1], candidate_ref.to_cbor())
        self.assertEqual(observed_member[7], bridge().to_cbor())

    def test_context_bridge_and_temporal_slots_are_closed_and_ordered(self) -> None:
        self.assertEqual(len(bridge().to_cbor()[0]), 10)
        self.assertEqual(len(bridge().to_cbor()[1]), 4)
        with self.assertRaises(AuthorityContractError):
            ContextSlotBridgeAttestationV2(
                slot_name="zone",
                source_value="not_applicable",
                reviewed_value="not_applicable",
                relation=ContextBridgeRelationV2.EXACT_MATCH,
                evidence_refs=(),
                rationale="missing evidence",
            )
        with self.assertRaises(AuthorityContractError):
            TemporalSlotAttestationV2(
                slot_name="trigger_order",
                reviewed_value="not_applicable",
                evidence_refs=(),
                rationale="missing evidence",
            )
        with self.assertRaises(AuthorityContractError):
            ContextSlotBridgeAttestationV2(
                slot_name="timing",
                source_value="not_applicable",
                reviewed_value="not_applicable",
                relation="unresolved",  # type: ignore[arg-type]
                evidence_refs=(),
                rationale="invalid",
            )

    def test_all_six_identity_vectors_are_shared_and_envelope_bound(self) -> None:
        matrix = json.loads(
            (
                ROOT
                / "conformance/fixtures/authority/"
                / "context_application_v2_identity_golden_matrix.v1.json"
            ).read_text(encoding="utf-8")
        )
        self.assertEqual(
            matrix["schema_version"], "context-application-v2-identity-golden-matrix.v1"
        )
        self.assertEqual(
            {entry["kind"] for entry in matrix["identities"]},
            {
                "context_application_v2",
                "context_application_record_v2",
                "context_supersession_v2",
                "context_supersession_record_v2",
                "acceptance_subject_v3",
                "review_acceptance_event_v3",
            },
        )
        for entry in matrix["identities"]:
            with self.subTest(kind=entry["kind"]):
                kind = AuthorityIdentityKind(entry["kind"])
                decoded = decode_canonical(bytes.fromhex(entry["payload_cbor_hex"]))
                identity = compute_authority_identity(kind, decoded)
                self.assertEqual(identity.as_text(), entry["identity"])
                self.assertEqual(identity.digest_bytes.hex(), entry["digest_hex"])
                self.assertEqual(
                    encode_canonical(identity.to_cbor()).hex(), entry["identity_cbor_hex"]
                )

    def test_structural_identity_and_wire_negatives_fail_closed(self) -> None:
        with self.assertRaises(AuthorityContractError):
            canonical_identity_input(
                AuthorityIdentityKind.CONTEXT_APPLICATION_V2,
                [CONTEXT_APPLICATION_INPUT_SCHEMA_V2, ZERO, [member().to_cbor()[0:1]]],
            )
        with self.assertRaises(AuthorityContractError):
            ContextApplicationMemberV2(
                candidate_id="synthetic-candidate",
                candidate_identity_digest_reference=replace(
                    digest_reference(), semantic_domain="not-candidate-identity"
                ),
                source_instance_id="si.v1/synthetic/0",
                candidate_universe_binding=member().candidate_universe_binding,
                context_binding_v1=member().context_binding_v1,
                precondition_attestations_v1=[],
                member_evidence_refs=(evidence(),),
                context_member_bridge_attestation_v2=bridge(),
            )
        with self.assertRaises(AuthorityContractError):
            ReviewEventRefV3(
                path="sources/m2_5/authorities/review_acceptance_events/v3/" + "00" * 32 + ".json",
                raw_sha256=ZERO,
                event_id="ae.v2/" + "00" * 32,
            )
        with self.assertRaises(AuthorityContractError):
            ApplicationHostBindingV2(
                application_kind="relation_application",
                application_semantic_id="cpa.v2/" + "00" * 32,  # type: ignore[arg-type]
                host_binding_claim_ids=("hbc.v1/" + "00" * 32,),
            )
        with self.assertRaises(AuthorityContractError):
            ContextAuthoritySourceBindingV2(
                artifact_role="unknown",
                path="a",
                schema=None,
                raw_sha256=ZERO,
            )
        with self.assertRaises(AuthorityContractError):
            ContextAuthoritySourceBindingV2(
                artifact_role="candidate_universe",
                path="sources/m2_5/closures/C/wrong.json",
                schema="manafold.m2.5.c.interaction-candidate-universe.v2",
                raw_sha256=ZERO,
            )
        with self.assertRaises(AuthorityContractError):
            ContextApplicationV2InputV1(ZERO, (member(), member()))

    def test_v3_revocation_subject_is_valid_and_requires_null_replacement(self) -> None:
        revocation_payload = [
            AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_SUPERSESSION_RECORD.value,
            ZERO,
            ZERO,
            None,
            "context_application_v2_record",
            None,
            "authority_revocation",
            [evidence().to_cbor()],
        ]
        subject = AcceptanceSubjectPayloadV3(
            subject_kind=AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_SUPERSESSION_RECORD,
            subject_payload=revocation_payload,
        )
        self.assertTrue(subject.identity().as_text().startswith("asp.v3/"))

        invalid_payload = list(revocation_payload)
        invalid_payload[6] = "semantic_correction"
        with self.assertRaises(AuthorityContractError):
            AcceptanceSubjectPayloadV3(
                subject_kind=AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_SUPERSESSION_RECORD,
                subject_payload=invalid_payload,
            ).identity()

    def test_v3_reviewer_bindings_use_full_canonical_cbor_order(self) -> None:
        roster_ref = ReviewerRosterRefV1(
            path="sources/m2_5/authorities/reviewer_rosters/v1/" + "00" * 32 + ".json",
            schema=REVIEWER_ROSTER_SCHEMA_V1,
            raw_sha256=ZERO,
        )
        base = ContextAuthoritySourceBindingV2(
            artifact_role="base_authority_v1",
            path="sources/m2_5/authorities/interaction_review_authority.v1.json",
            schema="manafold.m2.5.c.interaction-review-authority.v1",
            raw_sha256=ZERO,
        )
        roster = ContextAuthoritySourceBindingV2(
            artifact_role="reviewer_roster_leaf",
            path=roster_ref.path,
            schema=roster_ref.schema,
            raw_sha256=ZERO,
        )
        event = ReviewAcceptanceEventInputV3(
            subject_kind=AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD,
            subject_payload_digest_reference=subject_digest_reference(),
            reviewer_roster_ref=roster_ref,
            reviewer_role_bindings=(
                ReviewerRoleBindingV1(
                    reviewer_id="b",
                    roles=("architecture_maintainer", "rules_authority_maintainer"),
                ),
                ReviewerRoleBindingV1(
                    reviewer_id="aa",
                    roles=("architecture_maintainer", "rules_authority_maintainer"),
                ),
            ),
            review_mode=ReviewMode.MULTI_REVIEWER,
            source_binding_digests=tuple(
                sorted((base, roster), key=lambda item: encode_canonical(item.to_cbor()))
            ),
            review_evidence_refs=(
                AcceptanceEvidenceRefV1(
                    path="a",
                    raw_sha256=ZERO,
                    locator=("whole_artifact", None),
                ),
            ),
        )
        self.assertEqual(len(event.identity().digest_bytes), 32)

    def test_shared_matrix_covers_both_v3_acceptance_subject_kinds(self) -> None:
        matrix = json.loads(
            (
                ROOT
                / "conformance/fixtures/authority/"
                / "context_application_v2_identity_golden_matrix.v1.json"
            ).read_text(encoding="utf-8")
        )
        subject_entries = [
            entry for entry in matrix["identities"] if entry["kind"] == "acceptance_subject_v3"
        ]
        variants = {entry["subject_kind"] for entry in subject_entries}
        self.assertEqual(
            variants,
            {
                "context_application_v2_record",
                "context_application_v2_supersession_record",
            },
        )
        for entry in subject_entries:
            with self.subTest(subject_kind=entry["subject_kind"]):
                identity = compute_authority_identity(
                    AuthorityIdentityKind.ACCEPTANCE_SUBJECT_V3,
                    decode_canonical(bytes.fromhex(entry["payload_cbor_hex"])),
                )
                self.assertEqual(identity.as_text(), entry["identity"])

    def test_new_wire_schema_fixture_is_closed(self) -> None:
        schema = json.loads(
            (ROOT / "schemas/context-application-authority.v2.schema.json").read_text(
                encoding="utf-8"
            )
        )
        fixture = json.loads(
            (
                ROOT / "conformance/fixtures/authority/context_application_authority.v2.json"
            ).read_text(encoding="utf-8")
        )
        jsonschema.Draft202012Validator(schema).validate(fixture)
        invalid = dict(fixture)
        invalid["unexpected"] = True
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(invalid)

        invalid_candidate = copy.deepcopy(fixture)
        invalid_candidate["context_application_v2_records"][0]["members"][0]["candidate_identity"][
            "semantic_domain"
        ] = "not-candidate-identity"
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(invalid_candidate)

        event_schema = json.loads(
            (ROOT / "schemas/review-acceptance-event.v3.schema.json").read_text(encoding="utf-8")
        )
        event_fixture = json.loads(
            (ROOT / "conformance/fixtures/authority/review_acceptance_event.v3.json").read_text(
                encoding="utf-8"
            )
        )
        jsonschema.Draft202012Validator(event_schema).validate(event_fixture)
        invalid_event = dict(event_fixture)
        invalid_event["checklist_id"] = ACCEPTANCE_CHECKLIST_V2 + "-wrong"
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(event_schema).validate(invalid_event)

    def test_schema_keeps_the_complete_context_value_vocabulary(self) -> None:
        schema = json.loads(
            (ROOT / "schemas/context-application-authority.v2.schema.json").read_text(
                encoding="utf-8"
            )
        )
        fixture = json.loads(
            (
                ROOT / "conformance/fixtures/authority/context_application_authority.v2.json"
            ).read_text(encoding="utf-8")
        )
        trigger_slot = fixture["context_application_v2_records"][0]["members"][0][
            "context_member_attestation"
        ]["context_slot_attestations"][7]
        trigger_slot["source_value"] = "triggered_event"
        trigger_slot["reviewed_value"] = "triggered_event"
        jsonschema.Draft202012Validator(schema).validate(fixture)

    def test_source_role_registry_is_closed(self) -> None:
        self.assertIn("base_authority_v1", CONTEXT_AUTHORITY_SOURCE_ROLES_V2)
        self.assertIn("acceptance_event_leaf_v3", CONTEXT_AUTHORITY_SOURCE_ROLES_V2)
        self.assertNotIn("context_application_authority_v2", CONTEXT_AUTHORITY_SOURCE_ROLES_V2)

    def test_structural_dtos_cover_records_subjects_events_and_host_links(self) -> None:
        observed_member = member()
        application = ContextApplicationV2InputV1(ZERO, (observed_member,))
        event_ref = ReviewEventRefV3(
            path="sources/m2_5/authorities/review_acceptance_events/v3/" + "00" * 32 + ".json",
            raw_sha256=bytes.fromhex("22" * 32),
            event_id="ae.v3/" + "00" * 32,
        )
        theorem_record = AuthorityIdentityV1(
            AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
            ZERO,
        )
        record = ContextApplicationV2Record.from_parts(
            application_id=application.identity(),
            theorem_record_id=theorem_record,
            members=(observed_member,),
            review_event_ref_v3=event_ref,
        )
        record_subject = AcceptanceSubjectPayloadV3(
            subject_kind=AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD,
            subject_payload=record.acceptance_free_subject_payload(),
        )
        supersession = ContextApplicationV2SupersessionInputV2(
            superseded_record_id_bytes=record.record_id.digest_bytes,
            replacement_record_id_bytes=None,
            replacement_record_kind=None,
            reason_code=SupersessionReason.AUTHORITY_REVOCATION,
            source_evidence_refs=(evidence(),),
        )
        supersession_record = ContextApplicationV2SupersessionRecord.from_parts(
            supersession_id=supersession.identity(),
            superseded_record_id=record.record_id,
            replacement_record_id=None,
            reason_code=SupersessionReason.AUTHORITY_REVOCATION,
            source_evidence_refs=(evidence(),),
            review_event_ref_v3=event_ref,
        )
        roster_ref = ReviewerRosterRefV1(
            path="sources/m2_5/authorities/reviewer_rosters/v1/" + "00" * 32 + ".json",
            schema=REVIEWER_ROSTER_SCHEMA_V1,
            raw_sha256=ZERO,
        )
        base = ContextAuthoritySourceBindingV2(
            artifact_role="base_authority_v1",
            path="sources/m2_5/authorities/interaction_review_authority.v1.json",
            schema="manafold.m2.5.c.interaction-review-authority.v1",
            raw_sha256=ZERO,
        )
        roster = ContextAuthoritySourceBindingV2(
            artifact_role="reviewer_roster_leaf",
            path=roster_ref.path,
            schema=roster_ref.schema,
            raw_sha256=ZERO,
        )
        event = ReviewAcceptanceEventInputV3(
            subject_kind=record_subject.subject_kind,
            subject_payload_digest_reference=DigestReferenceV1.from_identity(
                record_subject.identity()
            ),
            reviewer_roster_ref=roster_ref,
            reviewer_role_bindings=(
                ReviewerRoleBindingV1(
                    reviewer_id="alice",
                    roles=("architecture_maintainer", "rules_authority_maintainer"),
                ),
            ),
            review_mode=ReviewMode.MULTI_REVIEWER,
            source_binding_digests=tuple(
                sorted((base, roster), key=lambda item: encode_canonical(item.to_cbor()))
            ),
            review_evidence_refs=(
                AcceptanceEvidenceRefV1(
                    path="a",
                    raw_sha256=ZERO,
                    locator=("whole_artifact", None),
                ),
            ),
        )
        leaf = ReviewAcceptanceEventLeafV3.from_input(event)
        host = ApplicationHostBindingV2(
            application_kind="context_application",
            application_semantic_id=application.identity(),
            host_binding_claim_ids=("hbc.v1/" + "00" * 32,),
        )
        authority = ContextApplicationAuthorityV2(
            base_authority_v1_binding=base,
            host_binding_authority_v2_binding=None,
            candidate_universe_binding=ContextAuthoritySourceBindingV2(
                artifact_role="candidate_universe",
                path="sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
                schema="manafold.m2.5.c.interaction-candidate-universe.v2",
                raw_sha256=ZERO,
            ),
            source_bindings=(base,),
            context_application_v2_records=(record,),
            context_application_v2_supersession_records=(supersession_record,),
            application_host_bindings_v2=(host,),
        )
        self.assertEqual(record.to_wire()["acceptance"]["decision"], "human_accepted")
        self.assertEqual(supersession_record.to_wire()["reason_code"], "authority_revocation")
        self.assertEqual(leaf.to_wire()["schema"], ACCEPTANCE_EVENT_SCHEMA_V3)
        self.assertEqual(host.to_cbor()[0], "context_application")
        self.assertEqual(authority.to_wire()["schema"], CONTEXT_AUTHORITY_SCHEMA_V2)
        with self.assertRaises(AuthorityContractError):
            ContextApplicationAuthorityV2(
                base_authority_v1_binding=base,
                host_binding_authority_v2_binding=None,
                candidate_universe_binding=authority.candidate_universe_binding,
                source_bindings=(base,),
                context_application_v2_records=(record, record),
                context_application_v2_supersession_records=(),
                application_host_bindings_v2=(host,),
            )
        with self.assertRaises(AuthorityContractError):
            ContextApplicationAuthorityV2(
                base_authority_v1_binding=base,
                host_binding_authority_v2_binding=None,
                candidate_universe_binding=authority.candidate_universe_binding,
                source_bindings=(base,),
                context_application_v2_records=(record,),
                context_application_v2_supersession_records=(),
                application_host_bindings_v2=(host, host),
            )

    def test_reviewed_divergence_is_a_closed_structural_relation(self) -> None:
        attestation = ContextSlotBridgeAttestationV2(
            slot_name="timing",
            source_value="not_applicable",
            reviewed_value="trigger_time",
            relation=ContextBridgeRelationV2.REVIEWED_DIVERGENCE,
            evidence_refs=(evidence(),),
            rationale="synthetic divergence shape",
        )
        self.assertEqual(attestation.to_cbor()[3], "reviewed_divergence")

    def test_multi_member_application_uses_digest_source_order_without_deduplication(self) -> None:
        first = member()
        second = replace(
            first,
            candidate_id="d",
            candidate_identity_digest_reference=replace(
                first.candidate_identity_digest_reference,
                digest_bytes=bytes.fromhex("01" * 32),
            ),
            source_instance_id="t",
        )
        ordered = ContextApplicationV2InputV1(ZERO, (first, second))
        self.assertEqual(len(ordered.to_cbor()[2]), 2)
        with self.assertRaises(AuthorityContractError):
            ContextApplicationV2InputV1(ZERO, (second, first))

    def test_historical_v1_golden_matrix_stays_v1_only(self) -> None:
        matrix = json.loads(
            (ROOT / "conformance/fixtures/authority/identity_golden_matrix.v1.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(len(matrix["identities"]), 17)
        self.assertTrue(
            all(
                entry["identity"].split("/", maxsplit=1)[0].endswith(".v1")
                for entry in matrix["identities"]
            )
        )


if __name__ == "__main__":
    unittest.main()
