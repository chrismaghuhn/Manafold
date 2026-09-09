from __future__ import annotations

import json
import unittest
from dataclasses import replace
from pathlib import Path

from mtgml.authority import (
    CANONICAL_CBOR_ID,
    DIGEST_ENVELOPE_ID,
    RELATION_APPLICATION_INPUT_SCHEMA_V2,
    RELATION_APPLICATION_RECORD_INPUT_SCHEMA_V2,
    SHA256_ID,
    AcceptanceSubjectKindV4,
    AuthorityContractError,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    DigestReferenceV1,
    EvidenceRefV1,
    ParticipantRoleBridgeEntryV1,
    ParticipantRoleBridgeV1,
    RelationApplicationMemberV2,
    RelationApplicationV2,
    RelationApplicationV2Record,
    ReviewEventRefV4,
    compute_authority_identity,
)
from mtgml.persistence import decode_canonical, encode_canonical

ROOT = Path(__file__).resolve().parents[2]
ZERO = bytes(32)


def candidate_identity(digest: bytes = b"c" * 32) -> DigestReferenceV1:
    return DigestReferenceV1(
        envelope_id=DIGEST_ENVELOPE_ID,
        algorithm_id=SHA256_ID,
        semantic_domain="manafold.m2.5.c.candidate-identity.v1",
        payload_codec_id=CANONICAL_CBOR_ID,
        input_schema_id="manafold.m2.5.c.candidate-identity-input.v1",
        digest_bytes=digest,
    )


def evidence() -> EvidenceRefV1:
    return EvidenceRefV1(
        authority_kind="model",
        path="sources/model.json",
        locator=("whole_artifact", None),
        raw_sha256=b"e" * 32,
    )


def bridge() -> ParticipantRoleBridgeV1:
    return ParticipantRoleBridgeV1(
        (
            ParticipantRoleBridgeEntryV1(
                position=0,
                participant_kind="requirement_family",
                semantic_ref="cap.mass_destruction",
                historical_source_role="ordered_participant",
                reviewed_role="source",
            ),
            ParticipantRoleBridgeEntryV1(
                position=1,
                participant_kind="requirement_family",
                semantic_ref="cap.death_trigger",
                historical_source_role="ordered_participant",
                reviewed_role="affected",
            ),
        )
    )


def relation_binding() -> list[object]:
    return [
        "cross_deck",
        "directional_binary",
        "directed",
        "cross_host",
        [
            [0, "source", "requirement_family", "cap.mass_destruction"],
            [1, "affected", "requirement_family", "cap.death_trigger"],
        ],
    ]


def member(
    *, digest: bytes = b"c" * 32, source_instance_id: str = "si/0"
) -> RelationApplicationMemberV2:
    return RelationApplicationMemberV2(
        candidate_id="CROSS_DECK|P3|cap.mass_destruction|cap.death_trigger|DIRECTIONAL_BINARY",
        candidate_identity_digest_reference=candidate_identity(digest),
        source_instance_id=source_instance_id,
        candidate_universe_binding=[
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            "manafold.m2.5.c.interaction-candidate-universe.v2",
            b"u" * 32,
        ],
        reviewed_relation_binding_v1=relation_binding(),
        participant_role_bridge_v1=bridge(),
        precondition_attestations_v1=[],
        member_evidence_refs=(evidence(),),
        member_proof_attestation_v1=["positive_interaction", [[0], None]],
    )


def golden_member() -> RelationApplicationMemberV2:
    return RelationApplicationMemberV2(
        candidate_id="c",
        candidate_identity_digest_reference=candidate_identity(b"d" * 32),
        source_instance_id="s",
        candidate_universe_binding=["u", "s", b"u" * 32],
        reviewed_relation_binding_v1=[
            "intra_deck",
            "declared_card_trigger",
            "none",
            "not_applicable",
            [[0, "source", "card", "x"]],
        ],
        participant_role_bridge_v1=ParticipantRoleBridgeV1(
            (
                ParticipantRoleBridgeEntryV1(
                    position=0,
                    participant_kind="card",
                    semantic_ref="x",
                    historical_source_role="ordered_participant",
                    reviewed_role="source",
                ),
            )
        ),
        precondition_attestations_v1=[],
        member_evidence_refs=(EvidenceRefV1("model", "a", ("whole_artifact", None), b"e" * 32),),
        member_proof_attestation_v1=["positive_interaction", [[], None]],
    )


def event_ref() -> ReviewEventRefV4:
    return ReviewEventRefV4(
        path="sources/m2_5/authorities/review_acceptance_events/v4/" + "a" * 64 + ".json",
        raw_sha256=b"a" * 32,
        event_id="ae.v4/" + "a" * 64,
    )


class RelationApplicationV2ContractTests(unittest.TestCase):
    def test_registry_contains_only_the_two_slice3_identity_families(self) -> None:
        expected = {
            AuthorityIdentityKind.RELATION_APPLICATION_V2: (
                "rpa.v2/",
                "manafold.m2.5.c.relation-application.v2",
                RELATION_APPLICATION_INPUT_SCHEMA_V2,
                4,
            ),
            AuthorityIdentityKind.RELATION_APPLICATION_RECORD_V2: (
                "rpar.v2/",
                "manafold.m2.5.c.relation-application-record.v2",
                RELATION_APPLICATION_RECORD_INPUT_SCHEMA_V2,
                3,
            ),
        }
        for kind, (prefix, domain, schema, _arity) in expected.items():
            identity = AuthorityIdentityV1(kind, ZERO)
            self.assertEqual(identity.prefix, prefix)
            self.assertEqual(identity.semantic_domain, domain)
            self.assertEqual(identity.input_schema_id, schema)

    def test_member_cbor_keeps_reviewed_binding_and_bridge_as_distinct_fields(self) -> None:
        observed = member().to_cbor()
        self.assertEqual(len(observed), 9)
        self.assertEqual(observed[4], relation_binding())
        self.assertEqual(observed[5], bridge().to_cbor())
        wire = member().to_wire()
        self.assertEqual(
            set(wire),
            {
                "candidate_id",
                "candidate_identity",
                "source_instance_id",
                "candidate_universe_binding",
                "relation_binding",
                "participant_role_bridge",
                "precondition_attestations",
                "member_evidence_refs",
                "member_proof_attestation",
            },
        )

    def test_application_member_order_is_candidate_digest_then_source_instance(self) -> None:
        first = member(digest=b"a" * 32, source_instance_id="z")
        second = member(digest=b"b" * 32, source_instance_id="a")
        application = RelationApplicationV2(
            theorem_record_id_bytes=ZERO,
            terminal_disposition="required_interaction",
            members=(first, second),
        )
        self.assertEqual(application.members, (first, second))
        with self.assertRaises(AuthorityContractError):
            RelationApplicationV2(
                theorem_record_id_bytes=ZERO,
                terminal_disposition="required_interaction",
                members=(second, first),
            )

    def test_duplicate_member_key_is_rejected(self) -> None:
        first = member(digest=b"a" * 32, source_instance_id="same")
        with self.assertRaises(AuthorityContractError):
            RelationApplicationV2(
                theorem_record_id_bytes=ZERO,
                terminal_disposition="required_interaction",
                members=(first, replace(first)),
            )

    def test_bridge_is_nonempty_and_structurally_closed(self) -> None:
        with self.assertRaises(AuthorityContractError):
            ParticipantRoleBridgeV1(())
        with self.assertRaises(AuthorityContractError):
            RelationApplicationMemberV2(
                **{
                    **member().__dict__,
                    "participant_role_bridge_v1": ParticipantRoleBridgeV1(
                        (
                            ParticipantRoleBridgeEntryV1(
                                1,
                                "requirement_family",
                                "cap.mass_destruction",
                                "ordered_participant",
                                "source",
                            ),
                        )
                    ),
                }
            )

    def test_record_identity_and_v4_subject_payload_are_acceptance_free(self) -> None:
        theorem_id = AuthorityIdentityV1(AuthorityIdentityKind.RELATION_THEOREM_RECORD, ZERO)
        application = RelationApplicationV2(
            theorem_record_id_bytes=theorem_id.digest_bytes,
            terminal_disposition="required_interaction",
            members=(member(),),
        )
        record = RelationApplicationV2Record.from_parts(
            application_id=application.identity(),
            theorem_record_id=theorem_id,
            terminal_disposition="required_interaction",
            members=application.members,
            review_event_ref_v4=event_ref(),
        )
        subject = record.acceptance_free_subject_payload()
        self.assertEqual(subject[0], AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD.value)
        self.assertEqual(subject[1], application.identity().digest_bytes)
        self.assertEqual(subject[2], theorem_id.digest_bytes)
        self.assertEqual(subject[3], "required_interaction")
        self.assertEqual(subject[4], [member().to_cbor()])
        self.assertEqual(
            record.record_id.kind, AuthorityIdentityKind.RELATION_APPLICATION_RECORD_V2
        )

    def test_golden_identity_fixture_matches_python(self) -> None:
        matrix = json.loads(
            (
                ROOT
                / "conformance/fixtures/authority/"
                / "relation_application_v2_identity_golden_matrix.v1.json"
            ).read_text(encoding="utf-8")
        )
        self.assertEqual(
            {entry["kind"] for entry in matrix["identities"]},
            {"relation_application_v2", "relation_application_record_v2"},
        )
        for entry in matrix["identities"]:
            with self.subTest(kind=entry["kind"]):
                kind = AuthorityIdentityKind(entry["kind"])
                payload = decode_canonical(bytes.fromhex(entry["payload_cbor_hex"]))
                identity = compute_authority_identity(kind, payload)
                self.assertEqual(identity.as_text(), entry["identity"])
                self.assertEqual(
                    encode_canonical(identity.to_cbor()).hex(), entry["identity_cbor_hex"]
                )

    def test_member_proof_wire_goldens_use_the_closed_v1_object_shapes(self) -> None:
        matrix = json.loads(
            (
                ROOT
                / "conformance/fixtures/authority/"
                / "relation_application_v2_wire_golden.v1.json"
            ).read_text(encoding="utf-8")
        )
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
        model_boundary = [
            "sources/m2_5/closures/C/declared_interaction_model.v2.json",
            "manafold.m2.5.c.declared-interaction-model.v2",
            b"m" * 32,
            ["coverage_scope", None],
        ]
        scope = [
            "declared-interaction-model.v2",
            "2",
            model_boundary,
            "undeclared_relation_shape",
            ["cross_deck", "directional_binary", "binary", "directed", 2],
            [evidence().to_cbor()],
        ]
        observed = {
            "positive_interaction": member().to_wire()["member_proof_attestation"],
            "positive_separation": replace(
                member(),
                member_proof_attestation_v1=["positive_separation", [coverages]],
            ).to_wire()["member_proof_attestation"],
            "model_bound_scope": replace(
                member(),
                member_proof_attestation_v1=["model_bound_scope", [scope]],
            ).to_wire()["member_proof_attestation"],
        }
        self.assertEqual(observed, matrix["proof_wire_goldens"])


if __name__ == "__main__":
    unittest.main()
