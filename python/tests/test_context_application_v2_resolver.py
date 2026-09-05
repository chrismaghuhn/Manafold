from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from authority_source_resolver import AuthoritySourceResolver, ResolutionError
from context_application_v2_resolver import (
    ContextApplicationV2ResolutionError,
    ContextApplicationV2Resolver,
    canonical_source_bindings,
    context_source_binding_from_wire,
    reconstruct_container_source_closure,
    reconstruct_event_source_closure,
    require_exact_source_set,
)
from mtgml.authority import (
    ACCEPTANCE_EVENT_SCHEMA_V3,
    CANONICAL_CBOR_ID,
    CONTEXT_AUTHORITY_SOURCE_ROLES_V2,
    DIGEST_ENVELOPE_ID,
    SHA256_ID,
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKindV3,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationAuthorityV2,
    ContextApplicationMemberV2,
    ContextApplicationV2InputV1,
    ContextApplicationV2Record,
    ContextApplicationV2SupersessionInputV2,
    ContextApplicationV2SupersessionRecord,
    ContextApplicationV2SupersessionRecordInputV1,
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
)
from mtgml.persistence import encode_canonical


def digest(value: bytes) -> bytes:
    return hashlib.sha256(value).digest()


def binding(
    role: str, path: str, schema: str | None, marker: int
) -> ContextAuthoritySourceBindingV2:
    return ContextAuthoritySourceBindingV2(role, path, schema, bytes([marker]) * 32)


BASE = binding(
    "base_authority_v1",
    "sources/m2_5/authorities/interaction_review_authority.v1.json",
    "manafold.m2.5.c.interaction-review-authority.v1",
    1,
)
MODEL = binding(
    "declared_model",
    "sources/m2_5/closures/C/declared_interaction_model.v2.json",
    "manafold.m2.5.c.declared-interaction-model.v2",
    2,
)
CANDIDATE = binding(
    "candidate_universe",
    "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
    "manafold.m2.5.c.interaction-candidate-universe.v2",
    3,
)
ROSTER = binding(
    "reviewer_roster_leaf",
    "sources/m2_5/authorities/reviewer_rosters/v1/" + "04" * 32 + ".json",
    "manafold.m2.5.c.reviewer-roster.v1",
    4,
)
B2_CATALOG = binding(
    "b2_catalog",
    "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
    "manafold.m2.5.b2.requirement-family-catalog.v1",
    5,
)
B2_CLASSIFICATIONS = binding(
    "b2_classifications",
    "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
    "manafold.m2.5.b2.card-semantic-classifications.v1",
    6,
)
B2_CLOSURE = binding(
    "b2_closure",
    "sources/m2_5/closures/B2/classification_closure.v1.json",
    "manafold.m2.5.b2.classification-closure.v1",
    7,
)
B1_CITATIONS = binding(
    "b1_final_citations",
    "sources/m2_5/closures/B1/official_authority_citations.v3.json",
    "manafold.m2.5.b1.official-authority-citations.v3",
    8,
)
B1_CLOSURE = binding(
    "b1_final_closure",
    "sources/m2_5/closures/B1/official_authority_citation_closure.v2.json",
    "manafold.m2.5.b1.official-authority-citation-closure.v2",
    9,
)
EVENT_LEAF = binding(
    "acceptance_event_leaf_v3",
    "sources/m2_5/authorities/review_acceptance_events/v3/" + "0a" * 32 + ".json",
    ACCEPTANCE_EVENT_SCHEMA_V3,
    10,
)


class ContextApplicationV2ResolverTests(unittest.TestCase):
    def test_event_closure_expands_b2_and_b1_and_is_canonical(self) -> None:
        result = reconstruct_event_source_closure(
            fixed_bindings=(BASE, MODEL, ROSTER),
            direct_bindings=(
                CANDIDATE,
                B2_CATALOG,
                B2_CLASSIFICATIONS,
                B2_CLOSURE,
                B1_CITATIONS,
                B1_CLOSURE,
            ),
            b2_evidence_roles=("b2_classifications",),
            b1_citation=True,
        )

        self.assertEqual(
            {item.artifact_role for item in result},
            {
                "base_authority_v1",
                "declared_model",
                "reviewer_roster_leaf",
                "candidate_universe",
                "b2_catalog",
                "b2_classifications",
                "b2_closure",
                "b1_final_citations",
                "b1_final_closure",
            },
        )
        encoded = [encode_canonical(item.to_cbor()) for item in result]
        self.assertEqual(encoded, sorted(encoded))

    def test_event_closure_rejects_acceptance_leaf_and_container_role(self) -> None:
        with self.assertRaises(ContextApplicationV2ResolutionError):
            reconstruct_event_source_closure(
                fixed_bindings=(BASE, MODEL, ROSTER),
                direct_bindings=(EVENT_LEAF,),
            )

        with self.assertRaises(ValueError):
            ContextAuthoritySourceBindingV2(
                "context_application_authority_v2",
                "sources/m2_5/authorities/context_application_authority/v2/x.json",
                None,
                bytes(32),
            )

    def test_container_closure_deduplicates_shared_event_source(self) -> None:
        result = reconstruct_container_source_closure(
            static_bindings=(BASE, CANDIDATE),
            event_leaf_bindings=(EVENT_LEAF,),
            event_closures=((BASE, MODEL, ROSTER), (BASE, MODEL, ROSTER)),
        )
        self.assertEqual(
            [item.artifact_role for item in result],
            [
                item.artifact_role
                for item in canonical_source_bindings(
                    (BASE, CANDIDATE, EVENT_LEAF, BASE, MODEL, ROSTER, BASE, MODEL, ROSTER)
                )
            ],
        )
        self.assertEqual(
            sum(item.artifact_role == "base_authority_v1" for item in result),
            1,
        )

    def test_exact_set_rejects_missing_extra_and_wrong_digest(self) -> None:
        expected = canonical_source_bindings((BASE, MODEL))
        require_exact_source_set(expected, expected)
        with self.assertRaises(ContextApplicationV2ResolutionError):
            require_exact_source_set((BASE,), expected)
        with self.assertRaises(ContextApplicationV2ResolutionError):
            require_exact_source_set((BASE, MODEL, ROSTER), expected)
        with self.assertRaises(ContextApplicationV2ResolutionError):
            require_exact_source_set(
                (BASE, binding("declared_model", MODEL.path, MODEL.schema, 99)), expected
            )

    def test_source_binding_wire_uses_closed_v2_shape(self) -> None:
        parsed = context_source_binding_from_wire(MODEL.to_wire())
        self.assertEqual(parsed, MODEL)
        self.assertEqual(len(CONTEXT_AUTHORITY_SOURCE_ROLES_V2), 18)
        with self.assertRaises(ContextApplicationV2ResolutionError):
            context_source_binding_from_wire(
                {
                    "artifact_role": "declared_model",
                    "path": "sources/m2_5/closures/C/declared_interaction_model.v2.json",
                    "schema": "wrong",
                    "raw_sha256": "00" * 32,
                }
            )

    def test_repository_source_resolution_verifies_bytes_and_schema(self) -> None:
        with tempfile.TemporaryDirectory() as raw_temp:
            repo = Path(raw_temp)
            path = repo / Path(*MODEL.path.split("/"))
            path.parent.mkdir(parents=True)
            raw = json.dumps(
                {"schema": MODEL.schema, "model_id": "declared-interaction-model"}
            ).encode("utf-8")
            path.write_bytes(raw)
            resolver = ContextApplicationV2Resolver(AuthoritySourceResolver(repo))
            resolved = resolver.resolve_source_binding(
                ContextAuthoritySourceBindingV2(
                    MODEL.artifact_role, MODEL.path, MODEL.schema, digest(raw)
                )
            )
            self.assertEqual(resolved.raw_bytes, raw)
            self.assertEqual(resolved.raw_sha256, digest(raw).hex())

            with self.assertRaises(ResolutionError):
                resolver.resolve_source_binding(
                    ContextAuthoritySourceBindingV2(
                        MODEL.artifact_role, MODEL.path, MODEL.schema, bytes(32)
                    )
                )

    def test_model_evidence_maps_to_exact_v2_binding(self) -> None:
        evidence = EvidenceRefV1(
            authority_kind="model",
            path=MODEL.path,
            locator=("whole_artifact", None),
            raw_sha256=MODEL.raw_sha256,
        )
        with tempfile.TemporaryDirectory() as raw_temp:
            repo = Path(raw_temp)
            path = repo / Path(*MODEL.path.split("/"))
            path.parent.mkdir(parents=True)
            raw = json.dumps({"schema": MODEL.schema}).encode("utf-8")
            path.write_bytes(raw)
            actual = ContextAuthoritySourceBindingV2(
                MODEL.artifact_role, MODEL.path, MODEL.schema, digest(raw)
            )
            resolver = ContextApplicationV2Resolver(AuthoritySourceResolver(repo))
            resolved = resolver.resolve_evidence(
                EvidenceRefV1(
                    evidence.authority_kind,
                    evidence.path,
                    evidence.locator,
                    actual.raw_sha256,
                )
            )
            self.assertEqual(resolved.binding, actual)

    def test_v3_event_leaf_resolves_structurally_and_verifies_raw_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as raw_temp:
            repo = Path(raw_temp)
            model_raw = json.dumps(
                {"schema": MODEL.schema, "model_id": "declared-interaction-model"}
            ).encode("utf-8")
            model_path = repo / Path(*MODEL.path.split("/"))
            model_path.parent.mkdir(parents=True)
            model_path.write_bytes(model_raw)

            roster_raw = json.dumps(
                {
                    "schema": ROSTER.schema,
                    "reviewers": [
                        {
                            "reviewer_id": "alice",
                            "roles": ["architecture_maintainer"],
                        }
                    ],
                }
            ).encode("utf-8")
            roster_relative = (
                "sources/m2_5/authorities/reviewer_rosters/v1/" + digest(roster_raw).hex() + ".json"
            )
            roster_path = repo / Path(*roster_relative.split("/"))
            roster_path.parent.mkdir(parents=True, exist_ok=True)
            roster_path.write_bytes(roster_raw)
            roster_binding = ContextAuthoritySourceBindingV2(
                ROSTER.artifact_role,
                roster_relative,
                ROSTER.schema,
                digest(roster_raw),
            )

            base_raw = json.dumps({"schema": BASE.schema}).encode("utf-8")
            base_path = repo / Path(*BASE.path.split("/"))
            base_path.parent.mkdir(parents=True, exist_ok=True)
            base_path.write_bytes(base_raw)
            base_binding = ContextAuthoritySourceBindingV2(
                BASE.artifact_role,
                BASE.path,
                BASE.schema,
                digest(base_raw),
            )

            event_input = ReviewAcceptanceEventInputV3(
                subject_kind=AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD,
                subject_payload_digest_reference=DigestReferenceV1(
                    DIGEST_ENVELOPE_ID,
                    SHA256_ID,
                    "manafold.m2.5.c.acceptance-subject-payload.v3",
                    CANONICAL_CBOR_ID,
                    "manafold.m2.5.c.acceptance-subject-payload-input.v3",
                    bytes(32),
                ),
                reviewer_roster_ref=ReviewerRosterRefV1(
                    roster_relative,
                    ROSTER.schema,
                    digest(roster_raw),
                ),
                reviewer_role_bindings=(
                    ReviewerRoleBindingV1("alice", ("architecture_maintainer",)),
                ),
                review_mode=ReviewMode.MULTI_REVIEWER,
                source_binding_digests=(base_binding, roster_binding),
                review_evidence_refs=(
                    AcceptanceEvidenceRefV1(
                        MODEL.path,
                        digest(model_raw),
                        ("whole_artifact", None),
                    ),
                ),
            )
            event_wire = ReviewAcceptanceEventLeafV3.from_input(event_input).to_wire()
            event_raw = (json.dumps(event_wire, separators=(",", ":")) + "\n").encode("utf-8")
            event_id = cast(str, event_wire["event_id"])
            event_path = (
                "sources/m2_5/authorities/review_acceptance_events/v3/"
                + event_id.removeprefix("ae.v3/")
                + ".json"
            )
            event_file = repo / Path(*event_path.split("/"))
            event_file.parent.mkdir(parents=True, exist_ok=True)
            event_file.write_bytes(event_raw)
            reference = ReviewEventRefV3(event_path, digest(event_raw), event_id)

            resolver = ContextApplicationV2Resolver(AuthoritySourceResolver(repo))
            resolved = resolver.resolve_review_event_leaf_v3(reference)
            self.assertEqual(resolved.event_id, event_id)
            self.assertEqual(resolved.event.source_binding_digests, (base_binding, roster_binding))
            self.assertEqual(resolved.event.review_evidence_refs[0].path, MODEL.path)

            tampered = copy.deepcopy(event_wire)
            tampered["event_id"] = "ae.v3/" + "01" * 32
            event_file.write_bytes((json.dumps(tampered) + "\n").encode("utf-8"))
            with self.assertRaises(ResolutionError):
                resolver.resolve_review_event_leaf_v3(reference)

    def test_supersession_event_closure_reconstructs_from_base_and_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as raw_temp:
            repo = Path(raw_temp)
            model_raw = json.dumps(
                {"schema": MODEL.schema, "model_id": "declared-interaction-model"}
            ).encode("utf-8")
            model_relative = MODEL.path
            model_path = repo / Path(*model_relative.split("/"))
            model_path.parent.mkdir(parents=True)
            model_path.write_bytes(model_raw)

            roster_raw = json.dumps(
                {
                    "schema": ROSTER.schema,
                    "reviewers": [
                        {
                            "reviewer_id": "alice",
                            "roles": ["architecture_maintainer"],
                        }
                    ],
                }
            ).encode("utf-8")
            roster_relative = (
                "sources/m2_5/authorities/reviewer_rosters/v1/" + digest(roster_raw).hex() + ".json"
            )
            roster_path = repo / Path(*roster_relative.split("/"))
            roster_path.parent.mkdir(parents=True, exist_ok=True)
            roster_path.write_bytes(roster_raw)
            roster_ref = ReviewerRosterRefV1(roster_relative, ROSTER.schema, digest(roster_raw))

            b1_citations_raw = json.dumps({"schema": B1_CITATIONS.schema}).encode("utf-8")
            b1_closure_raw = json.dumps({"schema": B1_CLOSURE.schema}).encode("utf-8")
            for source_binding, raw in (
                (B1_CITATIONS, b1_citations_raw),
                (B1_CLOSURE, b1_closure_raw),
            ):
                file_path = repo / Path(*source_binding.path.split("/"))
                file_path.parent.mkdir(parents=True, exist_ok=True)
                file_path.write_bytes(raw)
            citation_binding = ContextAuthoritySourceBindingV2(
                B1_CITATIONS.artifact_role,
                B1_CITATIONS.path,
                B1_CITATIONS.schema,
                digest(b1_citations_raw),
            )
            closure_binding = ContextAuthoritySourceBindingV2(
                B1_CLOSURE.artifact_role,
                B1_CLOSURE.path,
                B1_CLOSURE.schema,
                digest(b1_closure_raw),
            )
            base_raw = json.dumps(
                {
                    "schema": BASE.schema,
                    "model_binding": {
                        "path": MODEL.path,
                        "raw_sha256": digest(model_raw).hex(),
                        "model_id": "declared-interaction-model",
                        "model_version": "v2",
                    },
                    "source_bindings": [
                        {
                            "authority_kind": "b1_final",
                            "artifact_role": B1_CLOSURE.artifact_role,
                            "path": B1_CLOSURE.path,
                            "schema_or_null": B1_CLOSURE.schema,
                            "raw_sha256": digest(b1_closure_raw).hex(),
                        }
                    ],
                    "relation_proofs": [],
                    "relation_applications": [],
                    "domain_proofs": [],
                    "domain_applications": [],
                    "context_proofs": [],
                    "context_applications": [],
                    "supersession_records": [],
                }
            ).encode("utf-8")
            base_path = repo / Path(*BASE.path.split("/"))
            base_path.parent.mkdir(parents=True, exist_ok=True)
            base_path.write_bytes(base_raw)
            base_binding = ContextAuthoritySourceBindingV2(
                BASE.artifact_role, BASE.path, BASE.schema, digest(base_raw)
            )

            evidence = EvidenceRefV1(
                "b1_final",
                B1_CITATIONS.path,
                ("whole_artifact", None),
                digest(b1_citations_raw),
            )
            supersession_input = ContextApplicationV2SupersessionInputV2(
                superseded_record_id_bytes=bytes(32),
                replacement_record_id_bytes=None,
                replacement_record_kind=None,
                reason_code=SupersessionReason.AUTHORITY_REVOCATION,
                source_evidence_refs=(evidence,),
            )
            supersession_id = supersession_input.identity()
            event_id = "ae.v3/" + "00" * 32
            event_ref = ReviewEventRefV3(
                "sources/m2_5/authorities/review_acceptance_events/v3/" + "00" * 32 + ".json",
                bytes(32),
                event_id,
            )
            subject = ContextApplicationV2SupersessionRecord(
                record_id=ContextApplicationV2SupersessionRecordInputV1(
                    supersession_id.digest_bytes, event_ref
                ).identity(),
                supersession_id=supersession_id,
                superseded_record_id=AuthorityIdentityV1(
                    AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2, bytes(32)
                ),
                replacement_record_id=None,
                reason_code=SupersessionReason.AUTHORITY_REVOCATION,
                source_evidence_refs=(evidence,),
                review_event_ref_v3=event_ref,
            )

            resolver = ContextApplicationV2Resolver(
                AuthoritySourceResolver(repo), base_authority_binding=base_binding
            )
            actual = resolver.expected_acceptance_source_closure_v3(subject, roster_ref)
            self.assertEqual(
                {item.artifact_role for item in actual},
                {
                    "base_authority_v1",
                    "declared_model",
                    "reviewer_roster_leaf",
                    "b1_final_citations",
                    "b1_final_closure",
                },
            )
            self.assertIn(citation_binding, actual)
            self.assertIn(closure_binding, actual)

    def test_application_closure_includes_exact_candidate_and_rev3_provenance(self) -> None:
        sys.path.insert(0, str(ROOT / "python" / "tests"))
        from test_authority_source_resolver import AuthoritySourceResolverTests

        fixture = AuthoritySourceResolverTests()
        fixture.setUp()
        try:
            source_resolver, candidate_binding, candidate, _instance = (
                fixture._synthetic_binding_fixture()
            )
            model_path = fixture.repo / Path(*MODEL.path.split("/"))
            model_raw = model_path.read_bytes()
            roster_raw = json.dumps(
                {
                    "schema": ROSTER.schema,
                    "reviewers": [
                        {
                            "reviewer_id": "alice",
                            "roles": ["architecture_maintainer"],
                        }
                    ],
                }
            ).encode("utf-8")
            roster_relative = (
                "sources/m2_5/authorities/reviewer_rosters/v1/" + digest(roster_raw).hex() + ".json"
            )
            roster_path = fixture.repo / Path(*roster_relative.split("/"))
            roster_path.parent.mkdir(parents=True, exist_ok=True)
            roster_path.write_bytes(roster_raw)
            roster_ref = ReviewerRosterRefV1(roster_relative, ROSTER.schema, digest(roster_raw))

            base_document = {
                "schema": BASE.schema,
                "model_binding": {
                    "path": MODEL.path,
                    "raw_sha256": digest(model_raw).hex(),
                    "model_id": "declared-interaction-model.v2",
                    "model_version": "v2",
                },
                "source_bindings": [],
                "context_proofs": [
                    {
                        "record_id": {"digest_hex": "00" * 32},
                    }
                ],
            }
            base_raw = (json.dumps(base_document) + "\n").encode("utf-8")
            base_path = fixture.repo / Path(*BASE.path.split("/"))
            base_path.parent.mkdir(parents=True, exist_ok=True)
            base_path.write_bytes(base_raw)
            base_binding = ContextAuthoritySourceBindingV2(
                BASE.artifact_role, BASE.path, BASE.schema, digest(base_raw)
            )

            evidence = EvidenceRefV1(
                "model", MODEL.path, ("whole_artifact", None), digest(model_raw)
            )
            context = tuple(
                ContextSlotBridgeAttestationV2(
                    slot_name=name,
                    source_value="not_applicable",
                    reviewed_value="not_applicable",
                    relation=ContextBridgeRelationV2.EXACT_MATCH,
                    evidence_refs=(evidence,),
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
                    evidence_refs=(evidence,),
                    rationale="x",
                )
                for name in (
                    "trigger_order",
                    "dependency_order",
                    "duration",
                    "replacement_order",
                )
            )
            candidate_identity = cast(dict[str, str], candidate["candidate_identity"])
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
                source_instance_id=cast(str, _instance["source_instance_id"]),
                candidate_universe_binding=[
                    candidate_binding.path,
                    candidate_binding.schema_or_null,
                    candidate_binding.raw_sha256,
                ],
                context_binding_v1=[
                    "binary",
                    "symmetric",
                    [[0, "ordered_participant", "requirement_family", "family.a"]],
                    "same_host",
                ],
                precondition_attestations_v1=[],
                member_evidence_refs=(evidence,),
                context_member_bridge_attestation_v2=ContextMemberBridgeAttestationV2(
                    context=context,
                    temporal=temporal,
                ),
            )
            theorem_record_id = AuthorityIdentityV1(
                AuthorityIdentityKind.CONTEXT_THEOREM_RECORD, bytes(32)
            )
            application_id = ContextApplicationV2InputV1(
                theorem_record_id_bytes=theorem_record_id.digest_bytes,
                members=(member,),
            ).identity()
            event_ref = ReviewEventRefV3(
                "sources/m2_5/authorities/review_acceptance_events/v3/" + "00" * 32 + ".json",
                bytes(32),
                "ae.v3/" + "00" * 32,
            )
            subject = ContextApplicationV2Record.from_parts(
                application_id=application_id,
                theorem_record_id=theorem_record_id,
                members=(member,),
                review_event_ref_v3=event_ref,
            )
            resolver = ContextApplicationV2Resolver(
                source_resolver, base_authority_binding=base_binding
            )
            actual = resolver.expected_acceptance_source_closure_v3(subject, roster_ref)
            self.assertEqual(
                {item.artifact_role for item in actual},
                {
                    "base_authority_v1",
                    "declared_model",
                    "reviewer_roster_leaf",
                    "candidate_universe",
                    "rev3_candidate_census",
                },
            )
            self.assertEqual(
                sum(item.artifact_role == "candidate_universe" for item in actual),
                1,
            )
        finally:
            fixture.tearDown()

    def test_container_closure_adds_event_leaf_and_checks_top_level_projections(self) -> None:
        with tempfile.TemporaryDirectory() as raw_temp:
            repo = Path(raw_temp)
            model_raw = json.dumps({"schema": MODEL.schema}).encode("utf-8")
            model_path = repo / Path(*MODEL.path.split("/"))
            model_path.parent.mkdir(parents=True)
            model_path.write_bytes(model_raw)
            model_binding = ContextAuthoritySourceBindingV2(
                MODEL.artifact_role, MODEL.path, MODEL.schema, digest(model_raw)
            )
            roster_raw = json.dumps(
                {
                    "schema": ROSTER.schema,
                    "reviewers": [
                        {
                            "reviewer_id": "alice",
                            "roles": ["architecture_maintainer"],
                        }
                    ],
                }
            ).encode("utf-8")
            roster_path = (
                "sources/m2_5/authorities/reviewer_rosters/v1/" + digest(roster_raw).hex() + ".json"
            )
            roster_file = repo / Path(*roster_path.split("/"))
            roster_file.parent.mkdir(parents=True)
            roster_file.write_bytes(roster_raw)
            roster_ref = ReviewerRosterRefV1(roster_path, ROSTER.schema, digest(roster_raw))
            roster_binding = ContextAuthoritySourceBindingV2(
                "reviewer_roster_leaf", roster_path, ROSTER.schema, digest(roster_raw)
            )
            candidate_raw = json.dumps({"schema": CANDIDATE.schema}).encode("utf-8")
            candidate_file = repo / Path(*CANDIDATE.path.split("/"))
            candidate_file.parent.mkdir(parents=True, exist_ok=True)
            candidate_file.write_bytes(candidate_raw)
            candidate_binding = ContextAuthoritySourceBindingV2(
                CANDIDATE.artifact_role, CANDIDATE.path, CANDIDATE.schema, digest(candidate_raw)
            )
            base_raw = json.dumps(
                {
                    "schema": BASE.schema,
                    "model_binding": {
                        "path": MODEL.path,
                        "raw_sha256": digest(model_raw).hex(),
                        "model_id": "declared-interaction-model",
                        "model_version": "v2",
                    },
                    "source_bindings": [],
                    "context_proofs": [],
                }
            ).encode("utf-8")
            base_file = repo / Path(*BASE.path.split("/"))
            base_file.parent.mkdir(parents=True, exist_ok=True)
            base_file.write_bytes(base_raw)
            base_binding = ContextAuthoritySourceBindingV2(
                BASE.artifact_role, BASE.path, BASE.schema, digest(base_raw)
            )

            evidence = EvidenceRefV1(
                "model", MODEL.path, ("whole_artifact", None), digest(model_raw)
            )
            supersession_input = ContextApplicationV2SupersessionInputV2(
                bytes(32), None, None, SupersessionReason.AUTHORITY_REVOCATION, (evidence,)
            )
            supersession_id = supersession_input.identity()
            event_input = ReviewAcceptanceEventInputV3(
                AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_SUPERSESSION_RECORD,
                DigestReferenceV1(
                    DIGEST_ENVELOPE_ID,
                    SHA256_ID,
                    "manafold.m2.5.c.acceptance-subject-payload.v3",
                    CANONICAL_CBOR_ID,
                    "manafold.m2.5.c.acceptance-subject-payload-input.v3",
                    bytes(32),
                ),
                roster_ref,
                (ReviewerRoleBindingV1("alice", ("architecture_maintainer",)),),
                ReviewMode.MULTI_REVIEWER,
                canonical_source_bindings((base_binding, model_binding, roster_binding)),
                (AcceptanceEvidenceRefV1(MODEL.path, digest(model_raw), ("whole_artifact", None)),),
            )
            event_wire = ReviewAcceptanceEventLeafV3.from_input(event_input).to_wire()
            event_id = cast(str, event_wire["event_id"])
            event_path = (
                "sources/m2_5/authorities/review_acceptance_events/v3/"
                + event_id.removeprefix("ae.v3/")
                + ".json"
            )
            event_raw = (json.dumps(event_wire) + "\n").encode("utf-8")
            event_file = repo / Path(*event_path.split("/"))
            event_file.parent.mkdir(parents=True)
            event_file.write_bytes(event_raw)
            event_ref = ReviewEventRefV3(event_path, digest(event_raw), event_id)
            subject = ContextApplicationV2SupersessionRecord(
                record_id=ContextApplicationV2SupersessionRecordInputV1(
                    supersession_id.digest_bytes, event_ref
                ).identity(),
                supersession_id=supersession_id,
                superseded_record_id=AuthorityIdentityV1(
                    AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2, bytes(32)
                ),
                replacement_record_id=None,
                reason_code=SupersessionReason.AUTHORITY_REVOCATION,
                source_evidence_refs=(evidence,),
                review_event_ref_v3=event_ref,
            )
            container_sources = canonical_source_bindings(
                (base_binding, candidate_binding, model_binding, roster_binding)
            )
            container = ContextApplicationAuthorityV2(
                base_authority_v1_binding=base_binding,
                host_binding_authority_v2_binding=None,
                candidate_universe_binding=candidate_binding,
                source_bindings=canonical_source_bindings(
                    (
                        *container_sources,
                        ContextAuthoritySourceBindingV2(
                            "acceptance_event_leaf_v3",
                            event_path,
                            ACCEPTANCE_EVENT_SCHEMA_V3,
                            digest(event_raw),
                        ),
                    )
                ),
                context_application_v2_records=(),
                context_application_v2_supersession_records=(subject,),
                application_host_bindings_v2=(),
            )
            resolver = ContextApplicationV2Resolver(
                AuthoritySourceResolver(repo), base_authority_binding=base_binding
            )
            expected = resolver.expected_container_source_closure_v2(container)
            self.assertEqual(
                {item.artifact_role for item in expected},
                {
                    "base_authority_v1",
                    "candidate_universe",
                    "declared_model",
                    "reviewer_roster_leaf",
                    "acceptance_event_leaf_v3",
                },
            )
            self.assertEqual(
                sum(item.artifact_role == "acceptance_event_leaf_v3" for item in expected),
                1,
            )
            with self.assertRaises(ContextApplicationV2ResolutionError):
                resolver.expected_container_source_closure_v2(
                    replace(
                        container,
                        base_authority_v1_binding=ContextAuthoritySourceBindingV2(
                            BASE.artifact_role, BASE.path, BASE.schema, bytes(32)
                        ),
                    )
                )

    def test_shared_closure_golden_matrix_matches_python_algebra(self) -> None:
        matrix = json.loads(
            (
                ROOT
                / "conformance"
                / "fixtures"
                / "authority"
                / "context_application_v2_closure_golden_matrix.v1.json"
            ).read_text(encoding="utf-8")
        )
        self.assertEqual(
            matrix["schema"],
            "manafold.m2.5.c.context-application-v2-closure-golden-matrix.v1",
        )
        for case in matrix["event_cases"]:
            with self.subTest(case=case["case_id"]):
                actual = reconstruct_event_source_closure(
                    fixed_bindings=tuple(
                        context_source_binding_from_wire(item) for item in case["fixed_bindings"]
                    ),
                    direct_bindings=tuple(
                        context_source_binding_from_wire(item) for item in case["direct_bindings"]
                    ),
                    available_bindings=tuple(
                        context_source_binding_from_wire(item)
                        for item in case["available_bindings"]
                    ),
                    b2_evidence_roles=tuple(case["b2_evidence_roles"]),
                    b1_citation=case["b1_citation"],
                    host_bindings=tuple(
                        context_source_binding_from_wire(item) for item in case["host_bindings"]
                    ),
                )
                self.assertEqual(
                    [item.to_wire() for item in actual], case["expected_source_bindings"]
                )
        for case in matrix["container_cases"]:
            with self.subTest(case=case["case_id"]):
                actual = reconstruct_container_source_closure(
                    static_bindings=tuple(
                        context_source_binding_from_wire(item) for item in case["static_bindings"]
                    ),
                    event_leaf_bindings=tuple(
                        context_source_binding_from_wire(item)
                        for item in case["event_leaf_bindings"]
                    ),
                    event_closures=tuple(
                        tuple(context_source_binding_from_wire(item) for item in closure)
                        for closure in case["event_closures"]
                    ),
                    host_bindings=tuple(
                        context_source_binding_from_wire(item) for item in case["host_bindings"]
                    ),
                )
                self.assertEqual(
                    [item.to_wire() for item in actual], case["expected_source_bindings"]
                )


if __name__ == "__main__":
    unittest.main()
