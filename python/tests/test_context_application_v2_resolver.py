from __future__ import annotations

import copy
import hashlib
import inspect
import json
import sys
import tempfile
import unittest
from collections.abc import Callable
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
    AUTHORITY_SCHEMA_V1,
    CANONICAL_CBOR_ID,
    CONTEXT_AUTHORITY_SOURCE_ROLES_V2,
    DIGEST_ENVELOPE_ID,
    SHA256_ID,
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKind,
    AcceptanceSubjectKindV3,
    AcceptanceSubjectPayloadV1,
    AcceptanceSubjectPayloadV3,
    ApplicationHostBindingV2,
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
    ReviewAcceptanceEventInputV1,
    ReviewAcceptanceEventInputV3,
    ReviewAcceptanceEventLeafV1,
    ReviewAcceptanceEventLeafV3,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewEventRefV1,
    ReviewEventRefV3,
    ReviewMode,
    SourceBindingDigestV1,
    SupersessionReason,
    TemporalSlotAttestationV2,
    compute_authority_identity,
)
from mtgml.persistence import encode_canonical


def digest(value: bytes) -> bytes:
    return hashlib.sha256(value).digest()


def identity_wire(identity: AuthorityIdentityV1) -> dict[str, object]:
    return {
        "envelope_id": "mtgml.digest-envelope.v1",
        "algorithm_id": "sha-256",
        "semantic_domain": identity.semantic_domain,
        "payload_codec_id": "mtgml.canonical-cbor.v1",
        "input_schema_id": identity.input_schema_id,
        "digest_hex": identity.digest_bytes.hex(),
    }


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

    def test_member_source_helper_uses_all_v2_member_identity_fields(self) -> None:
        evidence = EvidenceRefV1(
            "model",
            MODEL.path,
            ("whole_artifact", None),
            bytes(32),
        )
        context = tuple(
            ContextSlotBridgeAttestationV2(
                slot_name=slot,
                source_value="not_applicable",
                reviewed_value="not_applicable",
                relation=ContextBridgeRelationV2.EXACT_MATCH,
                evidence_refs=(evidence,),
                rationale="synthetic",
            )
            for slot in (
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
                slot_name=slot,
                reviewed_value="not_applicable",
                evidence_refs=(evidence,),
                rationale="synthetic",
            )
            for slot in (
                "trigger_order",
                "dependency_order",
                "duration",
                "replacement_order",
            )
        )
        member = ContextApplicationMemberV2(
            candidate_id="synthetic-candidate",
            candidate_identity_digest_reference=DigestReferenceV1(
                DIGEST_ENVELOPE_ID,
                SHA256_ID,
                "manafold.m2.5.c.candidate-identity.v1",
                CANONICAL_CBOR_ID,
                "manafold.m2.5.c.candidate-identity-input.v1",
                bytes(32),
            ),
            source_instance_id="si.v1/synthetic/0",
            candidate_universe_binding=[CANDIDATE.path, CANDIDATE.schema, CANDIDATE.raw_sha256],
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
        resolver = ContextApplicationV2Resolver(AuthoritySourceResolver(Path.cwd()))
        with self.assertRaises(ResolutionError):
            resolver.resolve_member_source_instance(member)

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
            model_raw = (ROOT / Path(*MODEL.path.split("/"))).read_bytes()
            model_path = repo / Path(*MODEL.path.split("/"))
            model_path.parent.mkdir(parents=True)
            model_path.write_bytes(model_raw)

            roster_raw = json.dumps(
                {
                    "schema": ROSTER.schema,
                    "reviewers": [
                        {
                            "reviewer_id": "alice",
                            "roles": [
                                "architecture_maintainer",
                                "rules_authority_maintainer",
                            ],
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
            review_relative = "docs/review/host-binding.md"
            review_raw = b"accepted host-binding evidence\n"
            review_path = repo / Path(*review_relative.split("/"))
            review_path.parent.mkdir(parents=True, exist_ok=True)
            review_path.write_bytes(review_raw)

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
                        review_relative,
                        digest(review_raw),
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
            self.assertEqual(resolved.event.review_evidence_refs[0].path, review_relative)
            resolved_review_evidence = resolver.resolve_acceptance_evidence(
                resolved.event.review_evidence_refs[0]
            )
            self.assertIsNone(resolved_review_evidence.binding)
            self.assertNotIn(
                review_relative,
                {binding.path for binding in resolved.event.source_binding_digests},
            )

            def assert_old_identity_mutation(
                mutation: Callable[[dict[str, object]], None],
                expected_code: str,
                expected_message: str,
            ) -> None:
                mutated = copy.deepcopy(event_wire)
                mutation(mutated)
                mutated_raw = (json.dumps(mutated, separators=(",", ":")) + "\n").encode("utf-8")
                event_file.write_bytes(mutated_raw)
                mutated_reference = ReviewEventRefV3(
                    event_path,
                    digest(mutated_raw),
                    event_id,
                )
                with self.assertRaises(ContextApplicationV2ResolutionError) as caught:
                    resolver.resolve_review_event_leaf_v3(mutated_reference)
                self.assertEqual(caught.exception.code, expected_code)
                self.assertEqual(str(caught.exception), expected_message)

            assert_old_identity_mutation(
                lambda value: cast(dict[str, object], value).__setitem__("decision", "draft"),
                "V3_EVENT_DECISION_INVALID",
                "V3 acceptance event decision is not human_accepted",
            )
            assert_old_identity_mutation(
                lambda value: cast(dict[str, object], value).__setitem__(
                    "checklist_id", "interaction-authority-review-checklist.v1"
                ),
                "CHECKLIST_V2_MISMATCH",
                "V3 acceptance event checklist is not V2",
            )

            mutated_schema = copy.deepcopy(event_wire)
            mutated_schema["schema"] = "wrong"
            mutated_schema_raw = (json.dumps(mutated_schema, separators=(",", ":")) + "\n").encode(
                "utf-8"
            )
            event_file.write_bytes(mutated_schema_raw)
            mutated_schema_reference = ReviewEventRefV3(
                event_path,
                digest(mutated_schema_raw),
                event_id,
            )
            with self.assertRaises(ResolutionError) as caught:
                resolver.resolve_review_event_leaf_v3(mutated_schema_reference)
            self.assertEqual(caught.exception.code, "SCHEMA_MISMATCH")

            tampered_event_id = copy.deepcopy(event_wire)
            tampered_event_id["event_id"] = "ae.v3/" + "01" * 32
            tampered_event_id_raw = (
                json.dumps(tampered_event_id, separators=(",", ":")) + "\n"
            ).encode("utf-8")
            event_file.write_bytes(tampered_event_id_raw)
            tampered_event_id_reference = ReviewEventRefV3(
                event_path,
                digest(tampered_event_id_raw),
                event_id,
            )
            with self.assertRaises(ContextApplicationV2ResolutionError) as caught:
                resolver.resolve_review_event_leaf_v3(tampered_event_id_reference)
            self.assertEqual(caught.exception.code, "V3_EVENT_IDENTITY_INVALID")
            self.assertEqual(
                str(caught.exception),
                "V3 acceptance event ID differs from its reference",
            )

            assert_old_identity_mutation(
                lambda value: cast(dict[str, object], value).__setitem__(
                    "review_mode", "invalid_review_mode"
                ),
                "REVIEW_MODE_INVALID",
                (
                    "V3 acceptance event structural fields are invalid: "
                    "'invalid_review_mode' is not a valid ReviewMode"
                ),
            )

            missing_roster_input = ReviewAcceptanceEventInputV3(
                subject_kind=event_input.subject_kind,
                subject_payload_digest_reference=event_input.subject_payload_digest_reference,
                reviewer_roster_ref=event_input.reviewer_roster_ref,
                reviewer_role_bindings=event_input.reviewer_role_bindings,
                review_mode=event_input.review_mode,
                source_binding_digests=(base_binding,),
                review_evidence_refs=event_input.review_evidence_refs,
            )
            missing_roster_wire = ReviewAcceptanceEventLeafV3.from_input(
                missing_roster_input
            ).to_wire()
            missing_roster_raw = (
                json.dumps(missing_roster_wire, separators=(",", ":")) + "\n"
            ).encode("utf-8")
            missing_roster_id = cast(str, missing_roster_wire["event_id"])
            missing_roster_path = (
                "sources/m2_5/authorities/review_acceptance_events/v3/"
                + missing_roster_id.removeprefix("ae.v3/")
                + ".json"
            )
            missing_roster_file = repo / Path(*missing_roster_path.split("/"))
            missing_roster_file.parent.mkdir(parents=True, exist_ok=True)
            missing_roster_file.write_bytes(missing_roster_raw)
            missing_roster_reference = ReviewEventRefV3(
                missing_roster_path,
                digest(missing_roster_raw),
                missing_roster_id,
            )
            with self.assertRaises(ContextApplicationV2ResolutionError) as caught:
                resolver.resolve_review_event_leaf_v3(missing_roster_reference)
            self.assertEqual(caught.exception.code, "V3_EVENT_SOURCE_INVALID")

            tampered_semantics = copy.deepcopy(event_wire)
            tampered_semantics["review_mode"] = "solo_separate_self_review"
            tampered_semantics_raw = (json.dumps(tampered_semantics) + "\n").encode("utf-8")
            event_file.write_bytes(tampered_semantics_raw)
            tampered_semantics_reference = ReviewEventRefV3(
                event_path,
                digest(tampered_semantics_raw),
                event_id,
            )
            with self.assertRaises(ContextApplicationV2ResolutionError):
                resolver.resolve_review_event_leaf_v3(tampered_semantics_reference)

            event_file.write_bytes(event_raw)
            tampered = copy.deepcopy(event_wire)
            tampered["event_id"] = "ae.v3/" + "01" * 32
            event_file.write_bytes((json.dumps(tampered) + "\n").encode("utf-8"))
            with self.assertRaises(ResolutionError):
                resolver.resolve_review_event_leaf_v3(reference)

    def test_supersession_event_closure_reconstructs_from_base_and_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as raw_temp:
            repo = Path(raw_temp)
            model_raw = (ROOT / Path(*MODEL.path.split("/"))).read_bytes()
            model_document = cast(dict[str, object], json.loads(model_raw.decode("utf-8")))
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
                            "roles": [
                                "architecture_maintainer",
                                "rules_authority_maintainer",
                            ],
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

            base_raw = json.dumps(
                {
                    "schema": BASE.schema,
                    "model_binding": {
                        "path": MODEL.path,
                        "raw_sha256": digest(model_raw).hex(),
                        "model_id": model_document["model_id"],
                        "model_version": model_document["model_version"],
                    },
                    "source_bindings": [
                        {
                            "authority_kind": "model",
                            "artifact_role": MODEL.artifact_role,
                            "path": MODEL.path,
                            "schema_or_null": MODEL.schema,
                            "raw_sha256": digest(model_raw).hex(),
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
                "model", MODEL.path, ("whole_artifact", None), digest(model_raw)
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
                },
            )

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
            model_raw = (ROOT / Path(*MODEL.path.split("/"))).read_bytes()
            model_document = cast(dict[str, object], json.loads(model_raw.decode("utf-8")))
            model_path.write_bytes(model_raw)
            universe_path = fixture.repo / Path(*CANDIDATE.path.split("/"))
            universe_document = cast(
                dict[str, object], json.loads(universe_path.read_text(encoding="utf-8"))
            )
            input_bindings = cast(dict[str, object], universe_document["input_bindings"])
            declared_model_input = cast(dict[str, object], input_bindings["declared_model"])
            declared_model_input["raw_sha256"] = digest(model_raw).hex()
            universe_raw = (json.dumps(universe_document, indent=2) + "\n").encode("utf-8")
            universe_path.write_bytes(universe_raw)
            candidate_binding = SourceBindingDigestV1(
                "candidate_universe",
                CANDIDATE.path,
                CANDIDATE.schema,
                digest(universe_raw),
            )
            roster_raw = json.dumps(
                {
                    "schema": ROSTER.schema,
                    "reviewers": [
                        {
                            "reviewer_id": "alice",
                            "roles": [
                                "architecture_maintainer",
                                "rules_authority_maintainer",
                            ],
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

            model_binding_v1 = SourceBindingDigestV1(
                "declared_model", MODEL.path, MODEL.schema, digest(model_raw)
            )
            roster_binding_v1 = SourceBindingDigestV1(
                "reviewer_roster_leaf", roster_relative, ROSTER.schema, digest(roster_raw)
            )
            review_relative = "docs/review/context-theorem.md"
            review_raw = b"accepted context theorem evidence\n"
            review_path = fixture.repo / Path(*review_relative.split("/"))
            review_path.parent.mkdir(parents=True, exist_ok=True)
            review_path.write_bytes(review_raw)
            review_evidence = AcceptanceEvidenceRefV1(
                review_relative, digest(review_raw), ("whole_artifact", None)
            )
            source_evidence = EvidenceRefV1(
                "model", MODEL.path, ("whole_artifact", None), digest(model_raw)
            )
            subject_shape = {
                "arity": "unary",
                "directionality": "none",
                "participant_roles": [
                    {
                        "position": 0,
                        "role": "source",
                        "participant_kind": "card",
                        "semantic_ref": "card.synthetic",
                    }
                ],
                "host_relationship": "not_applicable",
            }
            theorem_id = compute_authority_identity(
                AuthorityIdentityKind.CONTEXT_THEOREM,
                [
                    "manafold.m2.5.c.context-proof-input.v1",
                    cast(str, model_document["model_id"]),
                    [
                        "unary",
                        "none",
                        [[0, "source", "card", "card.synthetic"]],
                        "not_applicable",
                    ],
                    ["not_applicable"] * 10,
                    ["not_applicable"] * 4,
                    [],
                    [],
                    [],
                ],
            )
            rationale = "synthetic accepted context theorem"
            acceptance_subject = AcceptanceSubjectPayloadV1(
                AcceptanceSubjectKind.CONTEXT_THEOREM_RECORD,
                [theorem_id.digest_bytes, [source_evidence.to_cbor()], rationale],
            )
            event_input_v1 = ReviewAcceptanceEventInputV1(
                subject_kind=AcceptanceSubjectKind.CONTEXT_THEOREM_RECORD,
                subject_payload_digest=acceptance_subject.identity().digest_bytes,
                reviewer_roster_ref=roster_ref,
                reviewer_role_bindings=(
                    ReviewerRoleBindingV1(
                        "alice",
                        ("architecture_maintainer", "rules_authority_maintainer"),
                    ),
                ),
                review_mode=ReviewMode.SOLO_SEPARATE_SELF_REVIEW,
                source_binding_digests=tuple(
                    sorted(
                        (model_binding_v1, roster_binding_v1),
                        key=lambda item: encode_canonical(item.to_cbor()),
                    )
                ),
                review_evidence_refs=(review_evidence,),
            )
            event_leaf_v1 = ReviewAcceptanceEventLeafV1.from_input(event_input_v1)
            event_raw_v1 = (json.dumps(event_leaf_v1.to_wire(), indent=2) + "\n").encode("utf-8")
            event_relative_v1 = (
                "sources/m2_5/authorities/review_acceptance_events/v1/"
                + event_leaf_v1.event_id.as_text().removeprefix("ae.v1/")
                + ".json"
            )
            event_path_v1 = fixture.repo / Path(*event_relative_v1.split("/"))
            event_path_v1.parent.mkdir(parents=True, exist_ok=True)
            event_path_v1.write_bytes(event_raw_v1)
            event_ref_v1 = ReviewEventRefV1(
                event_relative_v1, digest(event_raw_v1), event_leaf_v1.event_id.as_text()
            )
            event_binding_v1 = SourceBindingDigestV1(
                "acceptance_event_leaf",
                event_relative_v1,
                "manafold.m2.5.c.review-acceptance-event.v1",
                digest(event_raw_v1),
            )
            theorem_record_id = compute_authority_identity(
                AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
                [
                    "manafold.m2.5.c.context-proof-record-input.v1",
                    theorem_id.digest_bytes,
                    [source_evidence.to_cbor()],
                    event_ref_v1.to_cbor(),
                    rationale,
                ],
            )
            theorem = {
                "theorem_id": identity_wire(theorem_id),
                "record_id": identity_wire(theorem_record_id),
                "subject_shape": subject_shape,
                "context_dimensions": ["not_applicable"] * 10,
                "temporal_semantics": ["not_applicable"] * 4,
                "preconditions": [],
                "b2_boundary_refs": [],
                "b1_final_citation_refs": [],
                "source_evidence_refs": [source_evidence.to_wire()],
                "semantic_rationale": rationale,
                "acceptance": {
                    "decision": "human_accepted",
                    "review_event_ref": event_ref_v1.to_wire(),
                },
            }
            base_document = {
                "schema": AUTHORITY_SCHEMA_V1,
                "model_binding": {
                    "path": MODEL.path,
                    "raw_sha256": digest(model_raw).hex(),
                    "model_id": model_document["model_id"],
                    "model_version": model_document["model_version"],
                },
                "source_bindings": [
                    {
                        "authority_kind": "model",
                        "artifact_role": model_binding_v1.artifact_role,
                        "path": model_binding_v1.path,
                        "schema_or_null": model_binding_v1.schema_or_null,
                        "raw_sha256": model_binding_v1.raw_sha256.hex(),
                    },
                    {
                        "authority_kind": "reviewer_roster",
                        "artifact_role": roster_binding_v1.artifact_role,
                        "path": roster_binding_v1.path,
                        "schema_or_null": roster_binding_v1.schema_or_null,
                        "raw_sha256": roster_binding_v1.raw_sha256.hex(),
                    },
                    {
                        "authority_kind": "acceptance_event",
                        "artifact_role": event_binding_v1.artifact_role,
                        "path": event_binding_v1.path,
                        "schema_or_null": event_binding_v1.schema_or_null,
                        "raw_sha256": event_binding_v1.raw_sha256.hex(),
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

            host_authority_path = "sources/m2_5/authorities/interaction_review_authority.v2.json"
            host_authority_schema = "manafold.m2.5.c.interaction-review-authority.v2"
            host_authority_raw = json.dumps({"schema": host_authority_schema}).encode("utf-8")
            host_authority_file = fixture.repo / Path(*host_authority_path.split("/"))
            host_authority_file.parent.mkdir(parents=True, exist_ok=True)
            host_authority_file.write_bytes(host_authority_raw)
            host_authority_binding = ContextAuthoritySourceBindingV2(
                "host_binding_authority_v2",
                host_authority_path,
                host_authority_schema,
                digest(host_authority_raw),
            )
            claim_a_id = "hbc.v1/" + "a1" * 32
            claim_b_id = "hbc.v1/" + "b2" * 32
            claim_bindings: dict[str, ContextAuthoritySourceBindingV2] = {}
            for claim_id in (claim_a_id, claim_b_id):
                claim_path = (
                    "sources/m2_5/authorities/cross_deck_host_binding_claims/v1/"
                    + claim_id.removeprefix("hbc.v1/")
                    + ".json"
                )
                claim_schema = "manafold.m2.5.c.cross-deck-host-binding-claim-record.v1"
                claim_raw = json.dumps({"schema": claim_schema, "claim_id": claim_id}).encode(
                    "utf-8"
                )
                claim_file = fixture.repo / Path(*claim_path.split("/"))
                claim_file.parent.mkdir(parents=True, exist_ok=True)
                claim_file.write_bytes(claim_raw)
                claim_bindings[claim_id] = ContextAuthoritySourceBindingV2(
                    "host_binding_claim_record",
                    claim_path,
                    claim_schema,
                    digest(claim_raw),
                )

            candidate_v2_binding = ContextAuthoritySourceBindingV2(
                "candidate_universe",
                CANDIDATE.path,
                CANDIDATE.schema,
                candidate_binding.raw_sha256,
            )
            event_expected_a = resolver._expected_acceptance_source_closure_v3(
                subject,
                roster_ref,
                base_authority_binding=base_binding,
                host_bindings=(host_authority_binding, claim_bindings[claim_a_id]),
            )
            acceptance_subject = AcceptanceSubjectPayloadV3(
                AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD,
                subject.acceptance_free_subject_payload(),
            )
            event_input_v3 = ReviewAcceptanceEventInputV3(
                subject_kind=AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD,
                subject_payload_digest_reference=DigestReferenceV1(
                    DIGEST_ENVELOPE_ID,
                    SHA256_ID,
                    "manafold.m2.5.c.acceptance-subject-payload.v3",
                    CANONICAL_CBOR_ID,
                    "manafold.m2.5.c.acceptance-subject-payload-input.v3",
                    acceptance_subject.identity().digest_bytes,
                ),
                reviewer_roster_ref=roster_ref,
                reviewer_role_bindings=(
                    ReviewerRoleBindingV1(
                        "alice", ("architecture_maintainer", "rules_authority_maintainer")
                    ),
                ),
                review_mode=ReviewMode.MULTI_REVIEWER,
                source_binding_digests=event_expected_a,
                review_evidence_refs=(review_evidence,),
            )
            event_wire_v3 = ReviewAcceptanceEventLeafV3.from_input(event_input_v3).to_wire()
            event_id_v3 = cast(str, event_wire_v3["event_id"])
            event_path_v3 = (
                "sources/m2_5/authorities/review_acceptance_events/v3/"
                + event_id_v3.removeprefix("ae.v3/")
                + ".json"
            )
            event_raw_v3 = (json.dumps(event_wire_v3) + "\n").encode("utf-8")
            event_file_v3 = fixture.repo / Path(*event_path_v3.split("/"))
            event_file_v3.parent.mkdir(parents=True, exist_ok=True)
            event_file_v3.write_bytes(event_raw_v3)
            event_ref_v3 = ReviewEventRefV3(event_path_v3, digest(event_raw_v3), event_id_v3)
            record_a = ContextApplicationV2Record.from_parts(
                application_id=application_id,
                theorem_record_id=theorem_record_id,
                members=(member,),
                review_event_ref_v3=event_ref_v3,
            )
            link_a = ApplicationHostBindingV2(
                "context_application",
                application_id,
                (claim_a_id,),
            )
            link_b = ApplicationHostBindingV2(
                "context_application",
                AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_APPLICATION_V2, b"b" * 32),
                (claim_b_id,),
            )
            event_leaf_binding = ContextAuthoritySourceBindingV2(
                "acceptance_event_leaf_v3",
                event_path_v3,
                ACCEPTANCE_EVENT_SCHEMA_V3,
                digest(event_raw_v3),
            )
            container_a = ContextApplicationAuthorityV2(
                base_authority_v1_binding=base_binding,
                host_binding_authority_v2_binding=host_authority_binding,
                candidate_universe_binding=candidate_v2_binding,
                source_bindings=canonical_source_bindings(
                    (
                        base_binding,
                        candidate_v2_binding,
                        *event_expected_a,
                        event_leaf_binding,
                    )
                ),
                context_application_v2_records=(record_a,),
                context_application_v2_supersession_records=(),
                application_host_bindings_v2=(link_a,),
            )
            expected_container_a = resolver.expected_container_source_closure_v2(container_a)
            container_ab = replace(
                container_a,
                source_bindings=canonical_source_bindings(
                    (*container_a.source_bindings, claim_bindings[claim_b_id])
                ),
                application_host_bindings_v2=tuple(
                    sorted((link_a, link_b), key=lambda item: encode_canonical(item.to_cbor()))
                ),
            )
            expected_container_ab = resolver.expected_container_source_closure_v2(container_ab)
            resolved_event = resolver.resolve_review_event_leaf_v3(event_ref_v3)
            self.assertEqual(resolved_event.event.source_binding_digests, event_expected_a)
            self.assertEqual(
                expected_container_a,
                tuple(
                    binding
                    for binding in expected_container_ab
                    if binding != claim_bindings[claim_b_id]
                ),
            )
            self.assertIn(claim_bindings[claim_b_id], expected_container_ab)
            self.assertNotIn(
                claim_bindings[claim_b_id], resolved_event.event.source_binding_digests
            )
        finally:
            fixture.tearDown()

    def test_container_closure_adds_event_leaf_and_checks_top_level_projections(self) -> None:
        with tempfile.TemporaryDirectory() as raw_temp:
            repo = Path(raw_temp)
            model_raw = (ROOT / Path(*MODEL.path.split("/"))).read_bytes()
            model_document = cast(dict[str, object], json.loads(model_raw.decode("utf-8")))
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
            base_raw = (
                json.dumps(
                    {
                        "schema": BASE.schema,
                        "model_binding": {
                            "path": MODEL.path,
                            "raw_sha256": digest(model_raw).hex(),
                            "model_id": model_document["model_id"],
                            "model_version": model_document["model_version"],
                        },
                        "source_bindings": [
                            {
                                "authority_kind": "model",
                                "artifact_role": MODEL.artifact_role,
                                "path": MODEL.path,
                                "schema_or_null": MODEL.schema,
                                "raw_sha256": digest(model_raw).hex(),
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
                )
                + "\n"
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
            host_authority_path = "sources/m2_5/authorities/interaction_review_authority.v2.json"
            host_authority_schema = "manafold.m2.5.c.interaction-review-authority.v2"
            host_authority_raw = json.dumps({"schema": host_authority_schema}).encode("utf-8")
            host_authority_file = repo / Path(*host_authority_path.split("/"))
            host_authority_file.parent.mkdir(parents=True, exist_ok=True)
            host_authority_file.write_bytes(host_authority_raw)
            host_authority_binding = ContextAuthoritySourceBindingV2(
                "host_binding_authority_v2",
                host_authority_path,
                host_authority_schema,
                digest(host_authority_raw),
            )
            claim_id = "hbc.v1/" + "c3" * 32
            claim_path = (
                "sources/m2_5/authorities/cross_deck_host_binding_claims/v1/"
                + claim_id.removeprefix("hbc.v1/")
                + ".json"
            )
            claim_schema = "manafold.m2.5.c.cross-deck-host-binding-claim-record.v1"
            claim_raw = json.dumps({"schema": claim_schema, "claim_id": claim_id}).encode("utf-8")
            claim_file = repo / Path(*claim_path.split("/"))
            claim_file.parent.mkdir(parents=True, exist_ok=True)
            claim_file.write_bytes(claim_raw)
            claim_binding = ContextAuthoritySourceBindingV2(
                "host_binding_claim_record",
                claim_path,
                claim_schema,
                digest(claim_raw),
            )
            host_link = ApplicationHostBindingV2(
                "context_application",
                AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_APPLICATION_V2, b"c" * 32),
                (claim_id,),
            )
            container_sources = canonical_source_bindings(
                (
                    base_binding,
                    candidate_binding,
                    model_binding,
                    roster_binding,
                    host_authority_binding,
                    claim_binding,
                )
            )
            container = ContextApplicationAuthorityV2(
                base_authority_v1_binding=base_binding,
                host_binding_authority_v2_binding=host_authority_binding,
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
                application_host_bindings_v2=(host_link,),
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
                    "host_binding_authority_v2",
                    "host_binding_claim_record",
                    "acceptance_event_leaf_v3",
                },
            )
            self.assertEqual(
                sum(item.artifact_role == "acceptance_event_leaf_v3" for item in expected),
                1,
            )
            resolved_event = resolver.resolve_review_event_leaf_v3(event_ref)
            self.assertNotIn(claim_binding, resolved_event.event.source_binding_digests)
            self.assertEqual(resolver.validate_container_source_closure_v2(container), expected)
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

    def test_acceptance_evidence_resolves_without_v2_source_role(self) -> None:
        with tempfile.TemporaryDirectory() as raw_temp:
            repo = Path(raw_temp)
            relative = "docs/review/host-binding.md"
            raw = b"accepted host-binding evidence\n"
            path = repo / Path(*relative.split("/"))
            path.parent.mkdir(parents=True)
            path.write_bytes(raw)
            evidence = AcceptanceEvidenceRefV1(
                relative,
                digest(raw),
                ("whole_artifact", None),
            )
            resolver = ContextApplicationV2Resolver(AuthoritySourceResolver(repo))
            resolved = resolver.resolve_acceptance_evidence(evidence)
            self.assertEqual(resolved.artifact.raw_bytes, raw)

            model_evidence = AcceptanceEvidenceRefV1(
                MODEL.path,
                bytes(32),
                ("whole_artifact", None),
            )
            with self.assertRaises(ContextApplicationV2ResolutionError):
                resolver.source_binding_for_evidence(model_evidence)  # type: ignore[arg-type]

    def test_host_bindings_are_subject_specific_not_container_global(self) -> None:
        app_a = AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_APPLICATION_V2, b"a" * 32)
        app_b = AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_APPLICATION_V2, b"b" * 32)
        claim_a = "hbc.v1/" + "a1" * 32
        claim_b = "hbc.v1/" + "b2" * 32
        link_a = ApplicationHostBindingV2("context_application", app_a, (claim_a,))
        link_b = ApplicationHostBindingV2("context_application", app_b, (claim_b,))
        host_authority = binding(
            "host_binding_authority_v2",
            "sources/m2_5/authorities/interaction_review_authority.v2.json",
            "manafold.m2.5.c.interaction-review-authority.v2",
            11,
        )
        claim_binding_a = binding(
            "host_binding_claim_record",
            "sources/m2_5/authorities/cross_deck_host_binding_claims/v1/" + "a1" * 32 + ".json",
            "manafold.m2.5.c.cross-deck-host-binding-claim-record.v1",
            12,
        )
        claim_binding_b = binding(
            "host_binding_claim_record",
            "sources/m2_5/authorities/cross_deck_host_binding_claims/v1/" + "b2" * 32 + ".json",
            "manafold.m2.5.c.cross-deck-host-binding-claim-record.v1",
            13,
        )
        container = ContextApplicationAuthorityV2(
            base_authority_v1_binding=BASE,
            host_binding_authority_v2_binding=host_authority,
            candidate_universe_binding=CANDIDATE,
            source_bindings=canonical_source_bindings(
                (BASE, CANDIDATE, host_authority, claim_binding_a, claim_binding_b)
            ),
            context_application_v2_records=(),
            context_application_v2_supersession_records=(),
            application_host_bindings_v2=(link_a, link_b),
        )
        resolver = ContextApplicationV2Resolver(AuthoritySourceResolver(Path.cwd()))
        container_without_b = replace(
            container,
            source_bindings=canonical_source_bindings(
                (BASE, CANDIDATE, host_authority, claim_binding_a)
            ),
            application_host_bindings_v2=(link_a,),
        )
        subject_a_without_b = resolver._container_host_bindings_for_application(
            container_without_b, app_a.as_text(), container_without_b.source_bindings
        )
        subject_a = resolver._container_host_bindings_for_application(
            container, app_a.as_text(), container.source_bindings
        )
        subject_b = resolver._container_host_bindings_for_application(
            container, app_b.as_text(), container.source_bindings
        )
        self.assertEqual(subject_a_without_b, subject_a)
        self.assertEqual(subject_a, (host_authority, claim_binding_a))
        self.assertEqual(subject_b, (host_authority, claim_binding_b))
        self.assertNotIn(claim_binding_b, subject_a)
        self.assertNotIn(claim_binding_a, subject_b)
        self.assertEqual(
            resolver._container_host_bindings(container, container.source_bindings),
            canonical_source_bindings((host_authority, claim_binding_a, claim_binding_b)),
        )

    def test_event_resolver_does_not_accept_caller_selected_host_bindings(self) -> None:
        resolver = ContextApplicationV2Resolver(AuthoritySourceResolver(Path.cwd()))
        self.assertNotIn(
            "host_bindings",
            inspect.signature(resolver.expected_acceptance_source_closure_v3).parameters,
        )
        self.assertNotIn(
            "host_bindings",
            inspect.signature(resolver.validate_event_source_closure_v3).parameters,
        )

    def test_base_authority_is_v1_validated_before_dependency_walk(self) -> None:
        with tempfile.TemporaryDirectory() as raw_temp:
            repo = Path(raw_temp)
            model_relative = MODEL.path
            model_path = repo / Path(*model_relative.split("/"))
            model_path.parent.mkdir(parents=True)
            model_raw = (ROOT / Path(*model_relative.split("/"))).read_bytes()
            model_path.write_bytes(model_raw)
            model_digest = digest(model_raw)
            base_document = {
                "schema": BASE.schema,
                "model_binding": {
                    "path": model_relative,
                    "raw_sha256": model_digest.hex(),
                    "model_id": "declared-interaction-model.v2",
                    "model_version": "v2",
                },
                "source_bindings": [
                    {
                        "authority_kind": "model",
                        "artifact_role": "declared_model",
                        "path": model_relative,
                        "schema_or_null": MODEL.schema,
                        "raw_sha256": model_digest.hex(),
                    }
                ],
                "relation_proofs": [],
                "relation_applications": [],
                "domain_proofs": [],
                "domain_applications": [],
                "context_proofs": [{"record_id": {"digest_hex": "00" * 32}}],
                "context_applications": [],
                "supersession_records": [],
            }
            base_raw = (json.dumps(base_document) + "\n").encode("utf-8")
            base_path = repo / Path(*BASE.path.split("/"))
            base_path.parent.mkdir(parents=True)
            base_path.write_bytes(base_raw)
            base_binding = ContextAuthoritySourceBindingV2(
                BASE.artifact_role, BASE.path, BASE.schema, digest(base_raw)
            )
            resolver = ContextApplicationV2Resolver(
                AuthoritySourceResolver(repo), base_authority_binding=base_binding
            )
            with self.assertRaises(ResolutionError):
                resolver._base_context(None)


if __name__ == "__main__":
    unittest.main()
