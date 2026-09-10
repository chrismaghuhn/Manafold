from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.authority import (
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationMemberV3,
    ContextApplicationV3InputV1,
    ContextApplicationV3Record,
    ContextBridgeRelationV2,
    ContextMemberBridgeAttestationV2,
    ContextSlotBridgeAttestationV2,
    DigestReferenceV1,
    EvidenceRefV1,
    ReviewEventRefV4,
    TemporalSlotAttestationV2,
    compute_authority_identity,
)
from mtgml.persistence import decode_canonical, encode_canonical


def evidence() -> EvidenceRefV1:
    return EvidenceRefV1("model", "sources/model.json", ("whole_artifact", None), b"e" * 32)


def candidate_identity(digest: bytes = b"c" * 32) -> DigestReferenceV1:
    return DigestReferenceV1(
        "mtgml.digest-envelope.v1",
        "sha-256",
        "manafold.m2.5.c.candidate-identity.v1",
        "mtgml.canonical-cbor.v1",
        "manafold.m2.5.c.candidate-identity-input.v1",
        digest,
    )


def bridge() -> ContextMemberBridgeAttestationV2:
    context = tuple(
        ContextSlotBridgeAttestationV2(
            slot_name,
            "not_applicable",
            "not_applicable",
            ContextBridgeRelationV2.EXACT_MATCH,
            (evidence(),),
            "unchanged",
        )
        for slot_name in (
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
            slot_name,
            "not_applicable",
            (evidence(),),
            "unchanged",
        )
        for slot_name in (
            "trigger_order",
            "dependency_order",
            "duration",
            "replacement_order",
        )
    )
    return ContextMemberBridgeAttestationV2(context, temporal)


def member(
    *, digest: bytes = b"c" * 32, source_instance_id: str = "si/0"
) -> ContextApplicationMemberV3:
    return ContextApplicationMemberV3(
        candidate_id="CROSS_DECK|P3|cap.mass_destruction|cap.death_trigger|DIRECTIONAL_BINARY",
        candidate_identity_digest_reference=candidate_identity(digest),
        source_instance_id=source_instance_id,
        candidate_universe_binding=[
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            "manafold.m2.5.c.interaction-candidate-universe.v2",
            b"u" * 32,
        ],
        reviewed_context_binding_v1=[
            "binary",
            "directed",
            [
                [0, "source", "requirement_family", "cap.mass_destruction"],
                [1, "affected", "requirement_family", "cap.death_trigger"],
            ],
            "cross_host",
        ],
        relation_application_v2_id_bytes=b"r" * 32,
        precondition_attestations_v1=[],
        member_evidence_refs=(evidence(),),
        context_member_bridge_attestation_v2=bridge(),
    )


def event_ref() -> ReviewEventRefV4:
    event_id = "ae.v4/" + "a" * 64
    return ReviewEventRefV4(
        "sources/m2_5/authorities/review_acceptance_events/v4/" + "a" * 64 + ".json",
        b"a" * 32,
        event_id,
    )


class ContextApplicationV3ContractTests(unittest.TestCase):
    def test_record_rejects_application_id_not_recomputed_from_theorem_and_members(self) -> None:
        application = ContextApplicationV3InputV1(b"t" * 32, (member(),))
        other_application = ContextApplicationV3InputV1(
            b"t" * 32,
            (member(digest=b"d" * 32, source_instance_id="si/1"),),
        )
        with self.assertRaises(ValueError):
            ContextApplicationV3Record.from_parts(
                other_application.identity(),
                AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_THEOREM_RECORD, b"t" * 32),
                application.members,
                event_ref(),
            )

    def test_member_wire_is_closed_and_uses_typed_rpa_v2_id(self) -> None:
        wire = member().to_wire()
        self.assertEqual(
            set(wire),
            {
                "candidate_id",
                "candidate_identity",
                "source_instance_id",
                "candidate_universe_binding",
                "context_binding",
                "relation_application_v2_id",
                "precondition_attestations",
                "member_evidence_refs",
                "context_member_attestation",
            },
        )
        self.assertEqual(wire["relation_application_v2_id"], "rpa.v2/" + "72" * 32)

    def test_cpa_and_cpar_identities_are_versioned_and_canonical(self) -> None:
        first = member(digest=b"a" * 32, source_instance_id="z")
        second = member(digest=b"b" * 32, source_instance_id="a")
        application = ContextApplicationV3InputV1(
            theorem_record_id_bytes=b"t" * 32,
            members=(first, second),
        )
        self.assertEqual(application.identity().kind, AuthorityIdentityKind.CONTEXT_APPLICATION_V3)
        self.assertTrue(application.identity().as_text().startswith("cpa.v3/"))

        with self.assertRaises(ValueError):
            ContextApplicationV3InputV1(
                theorem_record_id_bytes=b"t" * 32,
                members=(second, first),
            )

        record = ContextApplicationV3Record.from_parts(
            application.identity(),
            AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_THEOREM_RECORD, b"t" * 32),
            (first, second),
            event_ref(),
        )
        self.assertEqual(record.record_id.kind, AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V3)
        self.assertTrue(record.record_id.as_text().startswith("cpar.v3/"))

    def test_identity_golden_matrix_matches_python(self) -> None:
        matrix = json.loads(
            (
                ROOT
                / "conformance"
                / "fixtures"
                / "authority"
                / "context_application_v3_identity_golden_matrix.v1.json"
            ).read_text(encoding="utf-8")
        )
        for entry in matrix["identities"]:
            with self.subTest(kind=entry["kind"]):
                payload = decode_canonical(bytes.fromhex(entry["payload_cbor_hex"]))
                kind = AuthorityIdentityKind(entry["kind"])
                identity = compute_authority_identity(kind, payload)
                self.assertEqual(identity.as_text(), entry["identity"])
                self.assertEqual(
                    encode_canonical(identity.to_cbor()).hex(), entry["identity_cbor_hex"]
                )


if __name__ == "__main__":
    unittest.main()
