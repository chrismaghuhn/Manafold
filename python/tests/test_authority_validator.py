from __future__ import annotations

import base64
import csv
import hashlib
import io
import json
import shutil
import sys
import tempfile
import unittest
import zipfile
from copy import deepcopy
from pathlib import Path
from types import MappingProxyType
from typing import cast

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from authority_source_resolver import (
    REV3_CENSUS_MEMBER,
    AuthoritySourceResolver,
    ResolutionError,
    ResolutionStatus,
    Rev3ArchiveStore,
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
    EvidenceRefV1,
    RecordKind,
    ReviewAcceptanceEventInputV1,
    ReviewAcceptanceEventLeafV1,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewerRosterV1,
    ReviewerV1,
    ReviewEventRefV1,
    ReviewMode,
    SourceBindingDigestV1,
    SupersessionReason,
    SupersessionRecordV1,
    compute_authority_identity,
)
from mtgml.persistence import (
    CANONICAL_CBOR_ID,
    DIGEST_ENVELOPE_ID,
    SHA256_ID,
    encode_canonical,
    encode_envelope,
    hash_envelope,
)

MODEL_PATH = "sources/m2_5/closures/C/declared_interaction_model.v2.json"
MODEL_SCHEMA = "manafold.m2.5.c.declared-interaction-model.v2"
ROSTER_ROOT = "sources/m2_5/authorities/reviewer_rosters/v1"
EVENT_ROOT = "sources/m2_5/authorities/review_acceptance_events/v1"


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def json_bytes(value: object) -> bytes:
    return (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode("utf-8")


def identity_wire(identity: object) -> dict[str, object]:
    typed = cast("AuthorityIdentityV1", identity)
    return {
        "envelope_id": "mtgml.digest-envelope.v1",
        "algorithm_id": "sha-256",
        "semantic_domain": typed.semantic_domain,
        "payload_codec_id": "mtgml.canonical-cbor.v1",
        "input_schema_id": typed.input_schema_id,
        "digest_hex": typed.digest_bytes.hex(),
    }


class AuthorityValidatorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.repo = Path(self.tempdir.name) / "repo"
        self.repo.mkdir()
        model_path = self.repo / Path(*MODEL_PATH.split("/"))
        model_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / MODEL_PATH, model_path)
        self.document = self._relation_theorem_document()
        self.resolver = AuthoritySourceResolver(self.repo)

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def _write(self, relative: str, raw: bytes) -> None:
        path = self.repo / Path(*relative.split("/"))
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)

    def _model_binding(self) -> SourceBindingDigestV1:
        raw = (self.repo / Path(*MODEL_PATH.split("/"))).read_bytes()
        return SourceBindingDigestV1(
            "declared_model", MODEL_PATH, MODEL_SCHEMA, bytes.fromhex(digest(raw))
        )

    def _new_acceptance(
        self,
        subject_kind: AcceptanceSubjectKind,
        subject_payload: list[object],
        evidence_name: str,
        extra_bindings: tuple[SourceBindingDigestV1, ...] = (),
    ) -> tuple[dict[str, object], SourceBindingDigestV1]:
        theorem = cast(dict[str, object], self.document["relation_proofs"][0])
        old_event_ref = cast(
            dict[str, object],
            cast(dict[str, object], theorem["acceptance"])["review_event_ref"],
        )
        old_event_path = self.repo / Path(*cast(str, old_event_ref["path"]).split("/"))
        old_event = cast(dict[str, object], json.loads(old_event_path.read_text(encoding="utf-8")))
        roster_record = cast(dict[str, object], old_event["reviewer_roster_ref"])
        roster_ref = ReviewerRosterRefV1(
            cast(str, roster_record["path"]),
            cast(str, roster_record["schema"]),
            bytes.fromhex(cast(str, roster_record["raw_sha256"])),
        )
        roster_binding = SourceBindingDigestV1(
            "reviewer_roster_leaf",
            roster_ref.path,
            roster_ref.schema,
            roster_ref.raw_sha256,
        )
        model_binding = self._model_binding()
        event_bindings = tuple(
            sorted(
                (model_binding, roster_binding, *extra_bindings),
                key=lambda binding: encode_canonical(binding.to_cbor()),
            )
        )
        review_path = f"docs/review/{evidence_name}.md"
        review_raw = f"accepted evidence for {evidence_name}\n".encode()
        self._write(review_path, review_raw)
        review_evidence = AcceptanceEvidenceRefV1(
            review_path,
            bytes.fromhex(digest(review_raw)),
            ("whole_artifact", None),
        )
        roles = cast(list[dict[str, object]], old_event["reviewer_role_bindings"])
        role_bindings = tuple(
            ReviewerRoleBindingV1(
                cast(str, item["reviewer_id"]),
                tuple(cast(str, role) for role in cast(list[object], item["roles"])),
            )
            for item in roles
        )
        subject = AcceptanceSubjectPayloadV1(subject_kind, subject_payload)
        event_input = ReviewAcceptanceEventInputV1(
            subject_kind=subject_kind,
            subject_payload_digest=subject.identity().digest_bytes,
            reviewer_roster_ref=roster_ref,
            reviewer_role_bindings=role_bindings,
            review_mode=ReviewMode.SOLO_SEPARATE_SELF_REVIEW,
            source_binding_digests=event_bindings,
            review_evidence_refs=(review_evidence,),
        )
        event_leaf = ReviewAcceptanceEventLeafV1.from_input(event_input)
        event_raw = json_bytes(event_leaf.to_wire())
        event_id = event_leaf.event_id.as_text()
        event_path = f"{EVENT_ROOT}/{event_id.removeprefix('ae.v1/')}.json"
        self._write(event_path, event_raw)
        event_ref = ReviewEventRefV1(
            event_path,
            bytes.fromhex(digest(event_raw)),
            event_id,
        )
        event_binding = SourceBindingDigestV1(
            "acceptance_event_leaf",
            event_path,
            ACCEPTANCE_EVENT_SCHEMA_V1,
            bytes.fromhex(digest(event_raw)),
        )
        return (
            {
                "decision": "human_accepted",
                "review_event_ref": {
                    "path": event_ref.path,
                    "raw_sha256": event_ref.raw_sha256.hex(),
                    "locator": {"kind": "event_id", "value": event_ref.event_id},
                },
            },
            event_binding,
        )

    @staticmethod
    def _source_binding_wire(
        binding: SourceBindingDigestV1, authority_kind: str
    ) -> dict[str, object]:
        return {
            "authority_kind": authority_kind,
            "artifact_role": binding.artifact_role,
            "path": binding.path,
            "schema_or_null": binding.schema_or_null,
            "raw_sha256": binding.raw_sha256.hex(),
        }

    def _synthetic_candidate_source(
        self,
    ) -> tuple[
        AuthoritySourceResolver, SourceBindingDigestV1, dict[str, object], dict[str, object]
    ]:
        candidate_id = "CROSS_DECK|P1|family.a|family.b|DIRECTIONAL_BINARY"
        source_columns = [
            "candidate_id",
            "model_id",
            "scope",
            "pair_id",
            "left_family_id",
            "right_family_id",
            "relation",
            "disposition",
            "disposition_reason",
            "supporting_requirement_ids",
        ]
        source_values = [
            candidate_id,
            "interaction-model.v1",
            "CROSS_DECK",
            "P1",
            "family.a",
            "family.b",
            "DIRECTIONAL_BINARY",
            "AMBIGUOUS_REQUIRES_REVIEW",
            "synthetic source binding",
            '["family.a", "family.b"]',
        ]
        csv_buffer = io.StringIO(newline="")
        writer = csv.writer(csv_buffer, lineterminator="\n")
        writer.writerow(source_columns)
        writer.writerow(source_values)
        census_raw = csv_buffer.getvalue().encode("utf-8")
        manifest = {
            "schema": "manafold.m2.5.rev3.package-manifest.v1",
            "entries": [
                {
                    "path": REV3_CENSUS_MEMBER,
                    "bytes": len(census_raw),
                    "sha256": digest(census_raw),
                }
            ],
            "manifest_excluded_paths": [],
            "manifest_excludes_self": True,
        }
        archive_buffer = io.BytesIO()
        with zipfile.ZipFile(archive_buffer, "w", compression=zipfile.ZIP_STORED) as archive:
            archive.writestr("Manafold_M2_5_Package_Manifest_REV3.json", json_bytes(manifest))
            archive.writestr(REV3_CENSUS_MEMBER, census_raw)
        archive_raw = archive_buffer.getvalue()
        archive = Rev3ArchiveStore.from_bytes(archive_raw, digest(archive_raw))
        source_binding = {
            "kind": "rev3",
            "archive_member": REV3_CENSUS_MEMBER,
            "archive_member_sha256": digest(census_raw),
            "row_ordinal": 0,
            "source_columns": source_columns,
            "source_values": source_values,
        }
        participant_refs = [
            {"participant_kind": "requirement_family", "semantic_ref": "family.a"},
            {"participant_kind": "requirement_family", "semantic_ref": "family.b"},
        ]
        candidate_identity_preimage = [
            ["rev3", None],
            ["cross_deck", None],
            ["directional_binary", None],
            [[[ref["participant_kind"], None], ref["semantic_ref"]] for ref in participant_refs],
            ["family.a", "family.b"],
            [
                ["rev3", None],
                [
                    REV3_CENSUS_MEMBER,
                    bytes.fromhex(digest(census_raw)),
                    0,
                    source_columns,
                    source_values,
                ],
            ],
        ]
        candidate_digest = hash_envelope(
            encode_envelope(
                "manafold.m2.5.c.candidate-identity.v1",
                "manafold.m2.5.c.candidate-identity-input.v1",
                encode_canonical(candidate_identity_preimage),
            )
        )
        candidate_identity = {
            "envelope_id": DIGEST_ENVELOPE_ID,
            "algorithm_id": SHA256_ID,
            "semantic_domain": "manafold.m2.5.c.candidate-identity.v1",
            "payload_codec_id": CANONICAL_CBOR_ID,
            "input_schema_id": "manafold.m2.5.c.candidate-identity-input.v1",
            "digest_hex": candidate_digest.hex(),
        }
        candidate = {
            "candidate_id": candidate_id,
            "candidate_identity": candidate_identity,
            "source_origin": "rev3",
            "scope": "cross_deck",
            "relation": "directional_binary",
            "participant_refs": participant_refs,
            "supporting_requirement_ids": ["family.a", "family.b"],
            "source_binding": source_binding,
            "reconciliation_status": "unchanged",
            "reconciliation_reason": "synthetic candidate source",
        }
        source_instance_id = (
            "si.v1/"
            + base64.urlsafe_b64encode(candidate_id.encode("utf-8")).decode("ascii").rstrip("=")
            + "/0"
        )
        instance = {
            "source_instance_id": source_instance_id,
            "candidate_id": candidate_id,
            "source_binding": source_binding,
            "participant_bindings": [
                {"role": "ordered_participant", "participant_ref": ref} for ref in participant_refs
            ],
            "source_context": {
                key: "not_applicable"
                for key in (
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
        model_raw = (self.repo / Path(*MODEL_PATH.split("/"))).read_bytes()
        universe = {
            "schema": "manafold.m2.5.c.interaction-candidate-universe.v2",
            "model_id": "declared-interaction-model.v2",
            "input_bindings": {
                "declared_model": {
                    "path": MODEL_PATH,
                    "raw_sha256": digest(model_raw),
                },
                "review_additions": {
                    "path": "sources/m2_5/closures/C/interaction_review_additions.v2.json",
                    "raw_sha256": "11" * 32,
                },
                "rev3_candidate_source": {
                    "archive_member": REV3_CENSUS_MEMBER,
                    "archive_member_sha256": digest(census_raw),
                    "source_package_sha256": digest(archive_raw),
                },
                "b2_artifacts": [
                    {
                        "path": "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
                        "raw_sha256": "22" * 32,
                    },
                    {
                        "path": "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
                        "raw_sha256": "33" * 32,
                    },
                    {
                        "path": "sources/m2_5/closures/B2/classification_closure.v1.json",
                        "raw_sha256": "44" * 32,
                    },
                ],
                "b1_final_artifacts": [
                    {
                        "path": "sources/m2_5/closures/B1/official_authority_citations.v3.json",
                        "raw_sha256": "55" * 32,
                    },
                    {
                        "path": (
                            "sources/m2_5/closures/B1/official_authority_citation_closure.v2.json"
                        ),
                        "raw_sha256": "66" * 32,
                    },
                ],
            },
            "candidate_count": 1,
            "candidate_reconciliation_counts": {
                "unchanged": 1,
                "stale_rev3_candidate": 0,
                "removed_not_interaction": 0,
                "merged_semantic_duplicate": 0,
                "new_targeted_higher_order_candidate": 0,
                "new_b2_derived": 0,
            },
            "source_instance_count": 1,
            "candidates": [candidate],
            "source_instances": [instance],
        }
        universe_raw = json_bytes(universe)
        self._write("sources/m2_5/closures/C/interaction_candidate_universe.v2.json", universe_raw)
        binding = SourceBindingDigestV1(
            "candidate_universe",
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            "manafold.m2.5.c.interaction-candidate-universe.v2",
            bytes.fromhex(digest(universe_raw)),
        )
        return (
            AuthoritySourceResolver(self.repo, rev3_archive=archive),
            binding,
            candidate,
            instance,
        )

    def _relation_theorem_document(self) -> dict[str, object]:
        model_binding = self._model_binding()
        model_raw = (self.repo / Path(*MODEL_PATH.split("/"))).read_bytes()
        model_evidence = EvidenceRefV1(
            "model", MODEL_PATH, ("whole_artifact", None), bytes.fromhex(digest(model_raw))
        )
        rationale = "synthetic accepted relation theorem"
        theorem_semantic_input = [
            "manafold.m2.5.c.relation-proof-input.v1",
            "declared-interaction-model.v2",
            "positive_interaction",
            "unary",
            "declared_card_trigger",
            "none",
            "not_applicable",
            [[0, "source", "card", "card.synthetic"]],
            [],
            [
                "positive_interaction",
                [
                    [[0, 0, "reads", [], None, None, []]],
                    [],
                    None,
                ],
            ],
            [],
            [],
        ]
        theorem_id = compute_authority_identity(
            AuthorityIdentityKind.RELATION_THEOREM,
            theorem_semantic_input,
        )

        review_raw = b"accepted by the synthetic conformance fixture\n"
        review_path = "docs/review/authority-validator.md"
        self._write(review_path, review_raw)
        review_evidence = AcceptanceEvidenceRefV1(
            review_path, bytes.fromhex(digest(review_raw)), ("whole_artifact", None)
        )

        roster = ReviewerRosterV1(
            reviewers=(
                ReviewerV1(
                    "alice",
                    ("architecture_maintainer", "rules_authority_maintainer"),
                ),
            )
        )
        roster_wire = {
            "schema": REVIEWER_ROSTER_SCHEMA_V1,
            "reviewers": [
                {
                    "reviewer_id": reviewer.reviewer_id,
                    "roles": list(reviewer.roles),
                }
                for reviewer in roster.reviewers
            ],
        }
        roster_raw = json_bytes(roster_wire)
        roster_digest = bytes.fromhex(digest(roster_raw))
        roster_path = f"{ROSTER_ROOT}/{roster_digest.hex()}.json"
        self._write(roster_path, roster_raw)
        roster_ref = ReviewerRosterRefV1(roster_path, REVIEWER_ROSTER_SCHEMA_V1, roster_digest)
        roster_binding = SourceBindingDigestV1(
            "reviewer_roster_leaf", roster_path, REVIEWER_ROSTER_SCHEMA_V1, roster_digest
        )
        event_bindings = tuple(
            sorted(
                (model_binding, roster_binding),
                key=lambda binding: encode_canonical(binding.to_cbor()),
            )
        )
        subject = AcceptanceSubjectPayloadV1(
            AcceptanceSubjectKind.RELATION_THEOREM_RECORD,
            [theorem_id.digest_bytes, [model_evidence.to_cbor()], rationale],
        )
        event_input = ReviewAcceptanceEventInputV1(
            subject_kind=AcceptanceSubjectKind.RELATION_THEOREM_RECORD,
            subject_payload_digest=subject.identity().digest_bytes,
            reviewer_roster_ref=roster_ref,
            reviewer_role_bindings=(
                ReviewerRoleBindingV1(
                    "alice",
                    ("architecture_maintainer", "rules_authority_maintainer"),
                ),
            ),
            review_mode=ReviewMode.SOLO_SEPARATE_SELF_REVIEW,
            source_binding_digests=event_bindings,
            review_evidence_refs=(review_evidence,),
        )
        event_leaf = ReviewAcceptanceEventLeafV1.from_input(event_input)
        event_raw = json_bytes(event_leaf.to_wire())
        event_id = event_leaf.event_id.as_text()
        event_path = f"{EVENT_ROOT}/{event_id.removeprefix('ae.v1/')}.json"
        self._write(event_path, event_raw)
        event_ref = ReviewEventRefV1(
            event_path,
            bytes.fromhex(digest(event_raw)),
            event_id,
        )
        record_id = compute_authority_identity(
            AuthorityIdentityKind.RELATION_THEOREM_RECORD,
            [
                "manafold.m2.5.c.relation-proof-record-input.v1",
                theorem_id.digest_bytes,
                [model_evidence.to_cbor()],
                event_ref.to_cbor(),
                rationale,
            ],
        )
        event_binding = SourceBindingDigestV1(
            "acceptance_event_leaf",
            event_path,
            ACCEPTANCE_EVENT_SCHEMA_V1,
            bytes.fromhex(digest(event_raw)),
        )
        root_bindings = (model_binding, roster_binding, event_binding)
        root_bindings = tuple(
            sorted(root_bindings, key=lambda binding: encode_canonical(binding.to_cbor()))
        )
        theorem = {
            "theorem_id": identity_wire(theorem_id),
            "record_id": identity_wire(record_id),
            "proof_kind": "positive_interaction",
            "subject": {
                "arity": "unary",
                "relation": "declared_card_trigger",
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
            },
            "preconditions": [],
            "proof_payload": {
                "kind": "positive_interaction",
                "causal_chain": [
                    {
                        "ordinal": 0,
                        "from_role_position": 0,
                        "operation": "reads",
                        "through_boundary_refs": [],
                        "event_or_effect_role_position": None,
                        "to_role_position": None,
                        "b1_final_citation_refs": [],
                    }
                ],
                "required_relation_channels": [],
                "class_projection_template": None,
            },
            "b2_boundary_refs": [],
            "b1_final_citation_refs": [],
            "source_evidence_refs": [
                {
                    "authority_kind": "model",
                    "path": MODEL_PATH,
                    "locator": {"kind": "whole_artifact"},
                    "raw_sha256": model_binding.raw_sha256.hex(),
                }
            ],
            "semantic_rationale": rationale,
            "acceptance": {
                "decision": "human_accepted",
                "review_event_ref": {
                    "path": event_path,
                    "raw_sha256": event_ref.raw_sha256.hex(),
                    "locator": {"kind": "event_id", "value": event_id},
                },
            },
        }
        return {
            "schema": AUTHORITY_SCHEMA_V1,
            "model_binding": {
                "path": MODEL_PATH,
                "raw_sha256": model_binding.raw_sha256.hex(),
                "model_id": "declared-interaction-model.v2",
                "model_version": "2",
            },
            "source_bindings": [
                self._source_binding_wire(
                    binding,
                    {
                        "declared_model": "model",
                        "reviewer_roster_leaf": "reviewer_roster",
                        "acceptance_event_leaf": "acceptance_event",
                    }[binding.artifact_role],
                )
                for binding in root_bindings
            ],
            "relation_proofs": [theorem],
            "relation_applications": [],
            "domain_proofs": [],
            "domain_applications": [],
            "context_proofs": [],
            "context_applications": [],
            "supersession_records": [],
        }

    def test_valid_relation_theorem_is_accepted(self) -> None:
        from authority_validator import AuthorityValidator

        result = AuthorityValidator(self.resolver).validate(self.document)

        self.assertTrue(result.valid)
        self.assertEqual(result.counts["relation_proofs"], 1)

    def test_theorem_identity_mismatch_fails_without_mutating_input(self) -> None:
        from authority_validator import AuthorityValidator

        candidate = deepcopy(self.document)
        cast(dict[str, object], candidate["relation_proofs"][0])["theorem_id"] = {
            **cast(
                dict[str, object],
                cast(dict[str, object], candidate["relation_proofs"][0])["theorem_id"],
            ),
            "digest_hex": "00" * 32,
        }
        before = deepcopy(candidate)
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(self.resolver).validate(candidate)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(context.exception.code, "THEOREM_IDENTITY_MISMATCH")
        self.assertEqual(candidate, before)

    def test_validator_state_resets_after_rejection(self) -> None:
        from authority_validator import AuthorityValidator

        validator = AuthorityValidator(self.resolver)
        invalid = deepcopy(self.document)
        invalid["schema"] = "wrong-schema"
        with self.assertRaises(ResolutionError):
            validator.validate(invalid)

        result = validator.validate(self.document)
        self.assertTrue(result.valid)

    def test_relation_member_binds_source_instance_participant_roles(self) -> None:
        from authority_validator import AuthorityValidator

        resolver, candidate_binding, candidate, instance = self._synthetic_candidate_source()
        resolved = resolver.resolve_candidate_source_instance(
            cast(str, candidate["candidate_id"]),
            cast(dict[str, object], candidate["candidate_identity"]),
            cast(str, instance["source_instance_id"]),
            candidate_binding,
        )
        participant_roles = [
            {
                "position": 0,
                "role": "source",
                "participant_kind": "requirement_family",
                "semantic_ref": "family.a",
            },
            {
                "position": 1,
                "role": "ordered_participant",
                "participant_kind": "requirement_family",
                "semantic_ref": "family.b",
            },
        ]
        subject = {
            "arity": "binary",
            "relation": "directional_binary",
            "directionality": "directed",
            "participant_roles": participant_roles,
            "host_relationship": "cross_host",
        }
        member = {
            "relation_binding": {
                "scope": "cross_deck",
                "relation": "directional_binary",
                "directionality": "directed",
                "host_relationship": "cross_host",
                "participant_bindings": participant_roles,
            },
            "member_proof_attestation": {"kind": "positive_separation"},
        }
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(resolver)._validate_relation_member_binding(
                member, {"subject": subject}, resolved, "relation member"
            )
        self.assertEqual(context.exception.code, "MEMBER_SOURCE_PARTICIPANT_BINDING_MISMATCH")

    def test_relation_member_rejects_source_direction_reversal(self) -> None:
        from authority_validator import AuthorityValidator

        resolver, candidate_binding, candidate, instance = self._synthetic_candidate_source()
        resolved = resolver.resolve_candidate_source_instance(
            cast(str, candidate["candidate_id"]),
            cast(dict[str, object], candidate["candidate_identity"]),
            cast(str, instance["source_instance_id"]),
            candidate_binding,
        )
        participant_roles = [
            {
                "position": 0,
                "role": "ordered_participant",
                "participant_kind": "requirement_family",
                "semantic_ref": "family.a",
            },
            {
                "position": 1,
                "role": "ordered_participant",
                "participant_kind": "requirement_family",
                "semantic_ref": "family.b",
            },
        ]
        subject = {
            "arity": "binary",
            "relation": "directional_binary",
            "directionality": "symmetric",
            "participant_roles": participant_roles,
            "host_relationship": "cross_host",
        }
        member = {
            "relation_binding": {
                "scope": "cross_deck",
                "relation": "directional_binary",
                "directionality": "symmetric",
                "host_relationship": "cross_host",
                "participant_bindings": participant_roles,
            },
            "member_proof_attestation": {"kind": "positive_separation"},
        }
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(resolver)._validate_relation_member_binding(
                member, {"subject": subject}, resolved, "relation member"
            )
        self.assertEqual(context.exception.code, "MEMBER_SOURCE_SHAPE_MISMATCH")

    def test_context_member_binds_all_source_context_values(self) -> None:
        from authority_validator import AuthorityValidator

        resolver, candidate_binding, candidate, instance = self._synthetic_candidate_source()
        resolved = resolver.resolve_candidate_source_instance(
            cast(str, candidate["candidate_id"]),
            cast(dict[str, object], candidate["candidate_identity"]),
            cast(str, instance["source_instance_id"]),
            candidate_binding,
        )
        participant_roles = [
            {
                "position": 0,
                "role": "ordered_participant",
                "participant_kind": "requirement_family",
                "semantic_ref": "family.a",
            },
            {
                "position": 1,
                "role": "ordered_participant",
                "participant_kind": "requirement_family",
                "semantic_ref": "family.b",
            },
        ]
        binding = {
            "arity": "binary",
            "directionality": "directed",
            "participant_roles": participant_roles,
            "host_relationship": "cross_host",
        }
        member = {"context_binding": binding}
        theorem = {"subject_shape": binding}
        validator = AuthorityValidator(resolver)
        validator._validate_context_member_binding(member, theorem, resolved, "context member")
        with self.assertRaises(ResolutionError) as context:
            validator._validate_context_values_against_source(
                ["battlefield"] + ["not_applicable"] * 9,
                resolved,
                "context member",
            )
        self.assertEqual(context.exception.code, "MEMBER_SOURCE_CONTEXT_MISMATCH")

    def test_domain_criterion_attestation_evidence_must_resolve(self) -> None:
        from authority_validator import AuthorityValidator, _SourceRegistry

        model_binding = self._model_binding()
        registry = _SourceRegistry(
            MappingProxyType({encode_canonical(model_binding.to_cbor()): model_binding})
        )
        validator = AuthorityValidator(self.resolver)
        validator._root_bindings = registry
        model_raw = (self.repo / Path(*MODEL_PATH.split("/"))).read_bytes()
        missing_pointer = {
            "authority_kind": "model",
            "path": MODEL_PATH,
            "locator": {"kind": "json_pointer", "value": "/missing-criterion"},
            "raw_sha256": digest(model_raw),
        }
        with self.assertRaises(ResolutionError) as context:
            validator._resolve_evidence_wire_list([missing_pointer], "criterion evidence")
        self.assertEqual(context.exception.code, "EVIDENCE_LOCATOR_UNRESOLVED")

    def test_boundary_acceptance_source_set_excludes_operational_b2_classifications(self) -> None:
        from authority_validator import AuthorityValidator, _SourceRegistry

        model_binding = self._model_binding()
        roster_binding = SourceBindingDigestV1(
            "reviewer_roster_leaf",
            "sources/m2_5/authorities/reviewer_rosters/v1/" + "00" * 32 + ".json",
            REVIEWER_ROSTER_SCHEMA_V1,
            bytes(32),
        )
        catalog = SourceBindingDigestV1(
            "b2_catalog",
            "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
            "manafold.m2.5.b2.requirement-family-catalog.v1",
            bytes.fromhex("11" * 32),
        )
        classifications = SourceBindingDigestV1(
            "b2_classifications",
            "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
            "manafold.m2.5.b2.card-semantic-classifications.v1",
            bytes.fromhex("22" * 32),
        )
        closure = SourceBindingDigestV1(
            "b2_closure",
            "sources/m2_5/closures/B2/classification_closure.v1.json",
            "manafold.m2.5.b2.classification-closure.v1",
            bytes.fromhex("33" * 32),
        )
        bindings = (model_binding, roster_binding, catalog, classifications, closure)
        validator = AuthorityValidator(self.resolver)
        validator._root_bindings = _SourceRegistry(
            MappingProxyType({encode_canonical(binding.to_cbor()): binding for binding in bindings})
        )
        source_set = validator._subject_bindings(
            {
                "b2_boundary_refs": [
                    {
                        "family_id": "cap.synthetic",
                        "precise_semantic_definition": "synthetic boundary",
                    }
                ]
            },
            model_binding,
            roster_binding,
        )
        self.assertIn(encode_canonical(catalog.to_cbor()), source_set)
        self.assertIn(encode_canonical(closure.to_cbor()), source_set)
        self.assertNotIn(encode_canonical(classifications.to_cbor()), source_set)

    def test_missing_rev3_archive_member_evidence_fails_closed(self) -> None:
        from authority_validator import AuthorityValidator, _SourceRegistry

        resolver, _, _, _ = self._synthetic_candidate_source()
        member_path = "inputs/deck_row_source_resolution_REV3.csv"
        binding = SourceBindingDigestV1("rev3_source", member_path, None, bytes(32))
        validator = AuthorityValidator(resolver)
        validator._root_bindings = _SourceRegistry(
            MappingProxyType({encode_canonical(binding.to_cbor()): binding})
        )
        evidence = {
            "authority_kind": "rev3",
            "path": member_path,
            "locator": {"kind": "archive_member", "value": member_path},
            "raw_sha256": "00" * 32,
        }
        with self.assertRaises(ResolutionError) as context:
            validator._resolve_evidence_wire_list([evidence], "criterion evidence")
        self.assertEqual(context.exception.code, "REV3_MEMBER_MISSING")

    def test_missing_model_source_binding_fails_closed(self) -> None:
        from authority_validator import AuthorityValidator

        candidate = deepcopy(self.document)
        candidate["source_bindings"] = [
            binding
            for binding in cast(list[dict[str, object]], candidate["source_bindings"])
            if binding["artifact_role"] != "declared_model"
        ]
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(self.resolver).validate(candidate)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(context.exception.code, "SOURCE_BINDING_CLOSURE_MISMATCH")

    def test_tampered_acceptance_event_bytes_fail_closed(self) -> None:
        from authority_validator import AuthorityValidator

        candidate = deepcopy(self.document)
        theorem = cast(dict[str, object], candidate["relation_proofs"][0])
        acceptance = cast(dict[str, object], theorem["acceptance"])
        event_ref = cast(dict[str, object], acceptance["review_event_ref"])
        event_path = self.repo / Path(*cast(str, event_ref["path"]).split("/"))
        event = cast(dict[str, object], json.loads(event_path.read_text(encoding="utf-8")))
        event["subject_payload_digest"] = "00" * 32
        changed_raw = json_bytes(event)
        event_path.write_bytes(changed_raw)
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(self.resolver).validate(candidate)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(context.exception.code, "SOURCE_DIGEST_MISMATCH")

    def test_acceptance_event_from_another_snapshot_fails_closed(self) -> None:
        from authority_validator import AuthorityValidator

        candidate = deepcopy(self.document)
        theorem = cast(dict[str, object], candidate["relation_proofs"][0])
        acceptance = cast(dict[str, object], theorem["acceptance"])
        event_ref = cast(dict[str, object], acceptance["review_event_ref"])
        event_ref["raw_sha256"] = "11" * 32
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(self.resolver).validate(candidate)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(context.exception.code, "SOURCE_DIGEST_MISMATCH")

    def test_roster_byte_tampering_fails_closed(self) -> None:
        from authority_validator import AuthorityValidator

        theorem = cast(dict[str, object], self.document["relation_proofs"][0])
        event_ref = cast(
            dict[str, object],
            cast(dict[str, object], theorem["acceptance"])["review_event_ref"],
        )
        event_path = self.repo / Path(*cast(str, event_ref["path"]).split("/"))
        event = cast(dict[str, object], json.loads(event_path.read_text(encoding="utf-8")))
        roster_ref = cast(dict[str, object], event["reviewer_roster_ref"])
        roster_path = self.repo / Path(*cast(str, roster_ref["path"]).split("/"))
        roster = cast(dict[str, object], json.loads(roster_path.read_text(encoding="utf-8")))
        cast(list[dict[str, object]], roster["reviewers"])[0]["roles"] = [
            "rules_authority_maintainer"
        ]
        roster_path.write_bytes(json_bytes(roster))
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(self.resolver).validate(self.document)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(context.exception.code, "SOURCE_DIGEST_MISMATCH")

    def test_cross_snapshot_model_binding_fails_closed(self) -> None:
        from authority_validator import AuthorityValidator

        candidate = deepcopy(self.document)
        model_binding = next(
            binding
            for binding in cast(list[dict[str, object]], candidate["source_bindings"])
            if binding["artifact_role"] == "declared_model"
        )
        model_binding["raw_sha256"] = "11" * 32
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(self.resolver).validate(candidate)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(context.exception.code, "SOURCE_BINDING_CLOSURE_MISMATCH")

    def test_wrong_source_evidence_authority_fails_closed(self) -> None:
        from authority_validator import AuthorityValidator

        candidate = deepcopy(self.document)
        evidence = cast(
            dict[str, object],
            cast(dict[str, object], candidate["relation_proofs"][0])["source_evidence_refs"][0],
        )
        evidence["authority_kind"] = "rev3"
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(self.resolver).validate(candidate)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(context.exception.code, "AUTHORITY_CONTRACT_INVALID")

    def test_unknown_b1_reference_fails_closed_without_inference(self) -> None:
        from authority_validator import AuthorityValidator

        candidate = deepcopy(self.document)
        theorem = cast(dict[str, object], candidate["relation_proofs"][0])
        theorem["b1_final_citation_refs"] = [
            {"authority_id": "comprehensive_rules", "citation_id": "CR 999"}
        ]
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(self.resolver).validate(candidate)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(context.exception.code, "SOURCE_BINDING_MISSING")

    def test_unknown_superseded_record_fails_closed(self) -> None:
        from authority_validator import AuthorityValidator

        candidate = deepcopy(self.document)
        candidate["supersession_records"] = [
            {
                "supersession_id": identity_wire(
                    AuthorityIdentityV1(AuthorityIdentityKind.RELATION_SUPERSESSION, bytes(32))
                ),
                "superseded_record_id": identity_wire(
                    AuthorityIdentityV1(AuthorityIdentityKind.RELATION_THEOREM_RECORD, bytes(32))
                ),
                "replacement_record_id": None,
                "superseded_record_kind": "relation_theorem_record",
                "replacement_record_kind": None,
                "reason_code": "authority_revocation",
                "source_evidence_refs": cast(dict[str, object], candidate["relation_proofs"][0])[
                    "source_evidence_refs"
                ],
                "acceptance": cast(dict[str, object], candidate["relation_proofs"][0])[
                    "acceptance"
                ],
            }
        ]
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(self.resolver).validate(candidate)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(context.exception.code, "RECORD_REFERENCE_INVALID")

    def test_supersession_record_kind_is_bound_to_identity_kind(self) -> None:
        from authority_validator import AuthorityValidator

        candidate = deepcopy(self.document)
        theorem = cast(dict[str, object], candidate["relation_proofs"][0])
        candidate["supersession_records"] = [
            {
                "supersession_id": identity_wire(
                    AuthorityIdentityV1(AuthorityIdentityKind.RELATION_SUPERSESSION, bytes(32))
                ),
                "superseded_record_id": theorem["record_id"],
                "replacement_record_id": None,
                "superseded_record_kind": "domain_theorem_record",
                "replacement_record_kind": None,
                "reason_code": "authority_revocation",
                "source_evidence_refs": theorem["source_evidence_refs"],
                "acceptance": theorem["acceptance"],
            }
        ]
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(self.resolver).validate(candidate)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(context.exception.code, "AUTHORITY_IDENTITY_INVALID")

    def test_application_disposition_must_match_theorem(self) -> None:
        from authority_validator import AuthorityValidator

        candidate = deepcopy(self.document)
        theorem = cast(dict[str, object], candidate["relation_proofs"][0])
        theorem_record = cast(dict[str, object], theorem["record_id"])
        theorem_record_digest = bytes.fromhex(cast(str, theorem_record["digest_hex"]))
        application_id = compute_authority_identity(
            AuthorityIdentityKind.RELATION_APPLICATION,
            [
                "manafold.m2.5.c.relation-application-input.v1",
                theorem_record_digest,
                "not_an_interaction_with_proof",
                [],
            ],
        )
        event_reference = cast(dict[str, object], theorem["acceptance"])["review_event_ref"]
        event_reference = cast(dict[str, object], event_reference)
        event_locator = cast(dict[str, object], event_reference["locator"])
        event_ref_cbor = ReviewEventRefV1(
            cast(str, event_reference["path"]),
            bytes.fromhex(cast(str, event_reference["raw_sha256"])),
            cast(str, event_locator["value"]),
        ).to_cbor()
        application_record_id = compute_authority_identity(
            AuthorityIdentityKind.RELATION_APPLICATION_RECORD,
            [
                "manafold.m2.5.c.relation-application-record-input.v1",
                application_id.digest_bytes,
                event_ref_cbor,
            ],
        )
        candidate["relation_applications"] = [
            {
                "application_id": identity_wire(application_id),
                "record_id": identity_wire(application_record_id),
                "theorem_record_id": theorem["record_id"],
                "terminal_disposition": "not_an_interaction_with_proof",
                "members": [],
                "acceptance": theorem["acceptance"],
            }
        ]
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(self.resolver).validate(candidate)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(context.exception.code, "APPLICATION_THEOREM_MISMATCH")

    def test_domain_theorem_identity_is_checked(self) -> None:
        from authority_validator import AuthorityValidator

        candidate = deepcopy(self.document)
        theorem = cast(dict[str, object], candidate["relation_proofs"][0])
        candidate["relation_proofs"] = []
        candidate["domain_proofs"] = [
            {
                "theorem_id": identity_wire(
                    AuthorityIdentityV1(AuthorityIdentityKind.DOMAIN_THEOREM, bytes(32))
                ),
                "record_id": identity_wire(
                    AuthorityIdentityV1(AuthorityIdentityKind.DOMAIN_THEOREM_RECORD, bytes(32))
                ),
                "review_domain": "triggers_and_lki",
                "applicability": "applicable",
                "criterion": [],
                "preconditions": [],
                "b2_boundary_refs": [],
                "b1_final_citation_refs": [],
                "source_evidence_refs": theorem["source_evidence_refs"],
                "semantic_rationale": "synthetic domain theorem",
                "acceptance": theorem["acceptance"],
            }
        ]
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(self.resolver).validate(candidate)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(context.exception.code, "THEOREM_IDENTITY_MISMATCH")

    def test_context_theorem_identity_is_checked(self) -> None:
        from authority_validator import AuthorityValidator

        candidate = deepcopy(self.document)
        theorem = cast(dict[str, object], candidate["relation_proofs"][0])
        candidate["relation_proofs"] = []
        candidate["context_proofs"] = [
            {
                "theorem_id": identity_wire(
                    AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_THEOREM, bytes(32))
                ),
                "record_id": identity_wire(
                    AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_THEOREM_RECORD, bytes(32))
                ),
                "subject_shape": {
                    "arity": "unary",
                    "directionality": "none",
                    "participant_roles": theorem["subject"]["participant_roles"],
                    "host_relationship": "not_applicable",
                },
                "context_dimensions": ["not_applicable"] * 10,
                "temporal_semantics": ["not_applicable"] * 4,
                "preconditions": [],
                "b2_boundary_refs": [],
                "b1_final_citation_refs": [],
                "source_evidence_refs": theorem["source_evidence_refs"],
                "semantic_rationale": "synthetic context theorem",
                "acceptance": theorem["acceptance"],
            }
        ]
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(self.resolver).validate(candidate)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(context.exception.code, "THEOREM_IDENTITY_MISMATCH")

    def test_valid_domain_theorem_uses_acceptance_subject_binding(self) -> None:
        from authority_validator import AuthorityValidator

        rationale = "synthetic accepted domain theorem"
        model_raw = (self.repo / Path(*MODEL_PATH.split("/"))).read_bytes()
        source_evidence = EvidenceRefV1(
            "model", MODEL_PATH, ("whole_artifact", None), bytes.fromhex(digest(model_raw))
        )
        theorem_id = compute_authority_identity(
            AuthorityIdentityKind.DOMAIN_THEOREM,
            [
                "manafold.m2.5.c.domain-proof-input.v1",
                "declared-interaction-model.v2",
                "triggers_and_lki",
                "applicable",
                [],
                [],
                [],
                [],
            ],
        )
        acceptance, event_binding = self._new_acceptance(
            AcceptanceSubjectKind.DOMAIN_THEOREM_RECORD,
            [theorem_id.digest_bytes, [source_evidence.to_cbor()], rationale],
            "domain-theorem",
        )
        event_ref = cast(dict[str, object], acceptance["review_event_ref"])
        event_locator = cast(dict[str, object], event_ref["locator"])
        record_id = compute_authority_identity(
            AuthorityIdentityKind.DOMAIN_THEOREM_RECORD,
            [
                "manafold.m2.5.c.domain-proof-record-input.v1",
                theorem_id.digest_bytes,
                [source_evidence.to_cbor()],
                ReviewEventRefV1(
                    cast(str, event_ref["path"]),
                    bytes.fromhex(cast(str, event_ref["raw_sha256"])),
                    cast(str, event_locator["value"]),
                ).to_cbor(),
                rationale,
            ],
        )
        theorem = {
            "theorem_id": identity_wire(theorem_id),
            "record_id": identity_wire(record_id),
            "review_domain": "triggers_and_lki",
            "applicability": "applicable",
            "criterion": [],
            "preconditions": [],
            "b2_boundary_refs": [],
            "b1_final_citation_refs": [],
            "source_evidence_refs": [
                {
                    "authority_kind": "model",
                    "path": MODEL_PATH,
                    "locator": {"kind": "whole_artifact"},
                    "raw_sha256": source_evidence.raw_sha256.hex(),
                }
            ],
            "semantic_rationale": rationale,
            "acceptance": acceptance,
        }
        candidate = deepcopy(self.document)
        candidate["relation_proofs"] = []
        candidate["domain_proofs"] = [theorem]
        candidate["source_bindings"] = [
            item
            for item in cast(list[dict[str, object]], self.document["source_bindings"])
            if item["artifact_role"] != "acceptance_event_leaf"
        ] + [self._source_binding_wire(event_binding, "acceptance_event")]
        result = AuthorityValidator(self.resolver).validate(candidate)
        self.assertTrue(result.valid)
        self.assertEqual(result.counts["domain_proofs"], 1)

    def test_valid_context_theorem_uses_four_field_subject_shape(self) -> None:
        from authority_validator import AuthorityValidator

        rationale = "synthetic accepted context theorem"
        model_raw = (self.repo / Path(*MODEL_PATH.split("/"))).read_bytes()
        source_evidence = EvidenceRefV1(
            "model", MODEL_PATH, ("whole_artifact", None), bytes.fromhex(digest(model_raw))
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
                "declared-interaction-model.v2",
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
        acceptance, event_binding = self._new_acceptance(
            AcceptanceSubjectKind.CONTEXT_THEOREM_RECORD,
            [theorem_id.digest_bytes, [source_evidence.to_cbor()], rationale],
            "context-theorem",
        )
        event_ref = cast(dict[str, object], acceptance["review_event_ref"])
        event_locator = cast(dict[str, object], event_ref["locator"])
        record_id = compute_authority_identity(
            AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
            [
                "manafold.m2.5.c.context-proof-record-input.v1",
                theorem_id.digest_bytes,
                [source_evidence.to_cbor()],
                ReviewEventRefV1(
                    cast(str, event_ref["path"]),
                    bytes.fromhex(cast(str, event_ref["raw_sha256"])),
                    cast(str, event_locator["value"]),
                ).to_cbor(),
                rationale,
            ],
        )
        theorem = {
            "theorem_id": identity_wire(theorem_id),
            "record_id": identity_wire(record_id),
            "subject_shape": subject_shape,
            "context_dimensions": ["not_applicable"] * 10,
            "temporal_semantics": ["not_applicable"] * 4,
            "preconditions": [],
            "b2_boundary_refs": [],
            "b1_final_citation_refs": [],
            "source_evidence_refs": [
                {
                    "authority_kind": "model",
                    "path": MODEL_PATH,
                    "locator": {"kind": "whole_artifact"},
                    "raw_sha256": source_evidence.raw_sha256.hex(),
                }
            ],
            "semantic_rationale": rationale,
            "acceptance": acceptance,
        }
        candidate = deepcopy(self.document)
        candidate["relation_proofs"] = []
        candidate["context_proofs"] = [theorem]
        candidate["source_bindings"] = [
            item
            for item in cast(list[dict[str, object]], self.document["source_bindings"])
            if item["artifact_role"] != "acceptance_event_leaf"
        ] + [self._source_binding_wire(event_binding, "acceptance_event")]
        result = AuthorityValidator(self.resolver).validate(candidate)
        self.assertTrue(result.valid)
        self.assertEqual(result.counts["context_proofs"], 1)

    def test_required_interaction_without_class_template_fails_closed(self) -> None:
        from authority_validator import AuthorityValidator

        theorem = cast(dict[str, object], self.document["relation_proofs"][0])
        theorem_record = cast(dict[str, object], theorem["record_id"])
        theorem_record_digest = bytes.fromhex(cast(str, theorem_record["digest_hex"]))
        application_id = compute_authority_identity(
            AuthorityIdentityKind.RELATION_APPLICATION,
            [
                "manafold.m2.5.c.relation-application-input.v1",
                theorem_record_digest,
                "required_interaction",
                [],
            ],
        )
        acceptance, event_binding = self._new_acceptance(
            AcceptanceSubjectKind.RELATION_APPLICATION_RECORD,
            [application_id.digest_bytes],
            "relation-application",
        )
        event_ref = cast(dict[str, object], acceptance["review_event_ref"])
        event_locator = cast(dict[str, object], event_ref["locator"])
        record_id = compute_authority_identity(
            AuthorityIdentityKind.RELATION_APPLICATION_RECORD,
            [
                "manafold.m2.5.c.relation-application-record-input.v1",
                application_id.digest_bytes,
                ReviewEventRefV1(
                    cast(str, event_ref["path"]),
                    bytes.fromhex(cast(str, event_ref["raw_sha256"])),
                    cast(str, event_locator["value"]),
                ).to_cbor(),
            ],
        )
        application = {
            "application_id": identity_wire(application_id),
            "record_id": identity_wire(record_id),
            "theorem_record_id": theorem["record_id"],
            "terminal_disposition": "required_interaction",
            "members": [],
            "acceptance": acceptance,
        }
        candidate = deepcopy(self.document)
        candidate["relation_applications"] = [application]
        candidate["source_bindings"] = [
            *cast(list[dict[str, object]], self.document["source_bindings"]),
            self._source_binding_wire(event_binding, "acceptance_event"),
        ]
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(self.resolver).validate(candidate)
        self.assertEqual(context.exception.code, "CLASS_PROJECTION_REQUIRED")

    def test_relation_application_resolves_candidate_and_source_instance(self) -> None:
        from authority_validator import AuthorityValidator

        resolver, candidate_binding, candidate_record, instance = self._synthetic_candidate_source()
        model_raw = (self.repo / Path(*MODEL_PATH.split("/"))).read_bytes()
        model_evidence = EvidenceRefV1(
            "model", MODEL_PATH, ("whole_artifact", None), bytes.fromhex(digest(model_raw))
        )
        participant_roles = [
            {
                "position": 0,
                "role": "ordered_participant",
                "participant_kind": "requirement_family",
                "semantic_ref": "family.a",
            },
            {
                "position": 1,
                "role": "ordered_participant",
                "participant_kind": "requirement_family",
                "semantic_ref": "family.b",
            },
        ]
        class_projection = {
            "arity": "binary",
            "directionality": "directed",
            "participant_roles": participant_roles,
            "host_relationship": "cross_host",
            "context_dimensions": ["not_applicable"] * 10,
            "temporal_semantics": ["not_applicable"] * 4,
            "b2_family_refs": [],
            "b2_boundary_refs": [],
            "b1_final_citation_refs": [],
        }
        candidate_evidence = {
            "authority_kind": "c_candidate",
            "path": candidate_binding.path,
            "locator": {"kind": "whole_artifact"},
            "raw_sha256": candidate_binding.raw_sha256.hex(),
        }
        precondition = {
            "precondition_id": "candidate-shape",
            "precondition_kind": "candidate_relation_shape",
            "payload": ["cross_deck", "directional_binary", "directed", "cross_host"],
        }
        theorem_rationale = "synthetic candidate-bound relation theorem"
        theorem_semantic_input = [
            "manafold.m2.5.c.relation-proof-input.v1",
            "declared-interaction-model.v2",
            "positive_interaction",
            "binary",
            "directional_binary",
            "directed",
            "cross_host",
            [
                [
                    item["position"],
                    item["role"],
                    item["participant_kind"],
                    item["semantic_ref"],
                ]
                for item in participant_roles
            ],
            [
                [
                    "candidate-shape",
                    [
                        "candidate_relation_shape",
                        ["cross_deck", "directional_binary", "directed", "cross_host"],
                    ],
                ]
            ],
            [
                "positive_interaction",
                [
                    [[0, 0, "reads", [], None, None, []]],
                    [],
                    [
                        "binary",
                        "directed",
                        [
                            [
                                item["position"],
                                item["role"],
                                item["participant_kind"],
                                item["semantic_ref"],
                            ]
                            for item in participant_roles
                        ],
                        "cross_host",
                        ["not_applicable"] * 10,
                        ["not_applicable"] * 4,
                        [],
                        [],
                        [],
                    ],
                ],
            ],
            [],
            [],
        ]
        theorem_id = compute_authority_identity(
            AuthorityIdentityKind.RELATION_THEOREM,
            theorem_semantic_input,
        )
        theorem_acceptance, theorem_event_binding = self._new_acceptance(
            AcceptanceSubjectKind.RELATION_THEOREM_RECORD,
            [theorem_id.digest_bytes, [model_evidence.to_cbor()], theorem_rationale],
            "candidate-theorem",
        )
        theorem_event_ref = cast(dict[str, object], theorem_acceptance["review_event_ref"])
        theorem_locator = cast(dict[str, object], theorem_event_ref["locator"])
        theorem_record_id = compute_authority_identity(
            AuthorityIdentityKind.RELATION_THEOREM_RECORD,
            [
                "manafold.m2.5.c.relation-proof-record-input.v1",
                theorem_id.digest_bytes,
                [model_evidence.to_cbor()],
                ReviewEventRefV1(
                    cast(str, theorem_event_ref["path"]),
                    bytes.fromhex(cast(str, theorem_event_ref["raw_sha256"])),
                    cast(str, theorem_locator["value"]),
                ).to_cbor(),
                theorem_rationale,
            ],
        )
        theorem = {
            "theorem_id": identity_wire(theorem_id),
            "record_id": identity_wire(theorem_record_id),
            "proof_kind": "positive_interaction",
            "subject": {
                "arity": "binary",
                "relation": "directional_binary",
                "directionality": "directed",
                "participant_roles": participant_roles,
                "host_relationship": "cross_host",
            },
            "preconditions": [precondition],
            "proof_payload": {
                "kind": "positive_interaction",
                "causal_chain": [
                    {
                        "ordinal": 0,
                        "from_role_position": 0,
                        "operation": "reads",
                        "through_boundary_refs": [],
                        "event_or_effect_role_position": None,
                        "to_role_position": None,
                        "b1_final_citation_refs": [],
                    }
                ],
                "required_relation_channels": [],
                "class_projection_template": class_projection,
            },
            "b2_boundary_refs": [],
            "b1_final_citation_refs": [],
            "source_evidence_refs": [
                {
                    "authority_kind": "model",
                    "path": MODEL_PATH,
                    "locator": {"kind": "whole_artifact"},
                    "raw_sha256": model_evidence.raw_sha256.hex(),
                }
            ],
            "semantic_rationale": theorem_rationale,
            "acceptance": theorem_acceptance,
        }
        member_class_equivalence = {
            "theorem_projection": class_projection,
            "member_projection": class_projection,
            "equal_positions": [
                "arity",
                "directionality",
                "participant_roles",
                "host_relationship",
                "context_dimensions",
                "temporal_semantics",
                "b2_family_refs",
                "b2_boundary_refs",
                "b1_final_citation_refs",
            ],
            "semantic_claim_relation": {
                "kind": "same_theorem_semantic_id",
                "theorem_semantic_digest": theorem_id.digest_bytes.hex(),
            },
            "evidence_refs": [candidate_evidence],
            "rationale": "candidate projection equivalence",
        }
        member = {
            "candidate_id": candidate_record["candidate_id"],
            "candidate_identity": candidate_record["candidate_identity"],
            "source_instance_id": instance["source_instance_id"],
            "candidate_universe_binding": {
                "path": candidate_binding.path,
                "schema": candidate_binding.schema_or_null,
                "raw_sha256": candidate_binding.raw_sha256.hex(),
            },
            "relation_binding": {
                "scope": "cross_deck",
                "relation": "directional_binary",
                "directionality": "directed",
                "host_relationship": "cross_host",
                "participant_bindings": participant_roles,
            },
            "precondition_attestations": [
                {
                    "precondition_id": "candidate-shape",
                    "observed_value": [
                        "cross_deck",
                        "directional_binary",
                        "directed",
                        "cross_host",
                    ],
                    "evidence_refs": [candidate_evidence],
                    "equivalence_rationale": "candidate shape matches source",
                }
            ],
            "member_evidence_refs": [candidate_evidence],
            "member_proof_attestation": {
                "kind": "positive_interaction",
                "causal_chain_ordinals": [0],
                "class_projection_equivalence": member_class_equivalence,
            },
        }
        member_array = [
            member["candidate_id"],
            [
                "mtgml.digest-envelope.v1",
                "sha-256",
                "manafold.m2.5.c.candidate-identity.v1",
                "mtgml.canonical-cbor.v1",
                "manafold.m2.5.c.candidate-identity-input.v1",
                bytes.fromhex(cast(str, candidate_record["candidate_identity"]["digest_hex"])),
            ],
            member["source_instance_id"],
            [
                candidate_binding.path,
                candidate_binding.schema_or_null,
                candidate_binding.raw_sha256,
            ],
            [
                "cross_deck",
                "directional_binary",
                "directed",
                "cross_host",
                [
                    [
                        item["position"],
                        item["role"],
                        item["participant_kind"],
                        item["semantic_ref"],
                    ]
                    for item in participant_roles
                ],
            ],
            [
                [
                    "candidate-shape",
                    [
                        "cross_deck",
                        "directional_binary",
                        "directed",
                        "cross_host",
                    ],
                    [
                        [
                            "c_candidate",
                            candidate_binding.path,
                            ["whole_artifact", None],
                            candidate_binding.raw_sha256,
                        ]
                    ],
                    "candidate shape matches source",
                ]
            ],
            [
                [
                    "c_candidate",
                    candidate_binding.path,
                    ["whole_artifact", None],
                    candidate_binding.raw_sha256,
                ]
            ],
            [
                "positive_interaction",
                [
                    [0],
                    [
                        [
                            "binary",
                            "directed",
                            [
                                [
                                    item["position"],
                                    item["role"],
                                    item["participant_kind"],
                                    item["semantic_ref"],
                                ]
                                for item in participant_roles
                            ],
                            "cross_host",
                            ["not_applicable"] * 10,
                            ["not_applicable"] * 4,
                            [],
                            [],
                            [],
                        ],
                        [
                            "binary",
                            "directed",
                            [
                                [
                                    item["position"],
                                    item["role"],
                                    item["participant_kind"],
                                    item["semantic_ref"],
                                ]
                                for item in participant_roles
                            ],
                            "cross_host",
                            ["not_applicable"] * 10,
                            ["not_applicable"] * 4,
                            [],
                            [],
                            [],
                        ],
                        [
                            "arity",
                            "directionality",
                            "participant_roles",
                            "host_relationship",
                            "context_dimensions",
                            "temporal_semantics",
                            "b2_family_refs",
                            "b2_boundary_refs",
                            "b1_final_citation_refs",
                        ],
                        ["same_theorem_semantic_id", theorem_id.digest_bytes],
                        [
                            [
                                "c_candidate",
                                candidate_binding.path,
                                ["whole_artifact", None],
                                candidate_binding.raw_sha256,
                            ]
                        ],
                        "candidate projection equivalence",
                    ],
                ],
            ],
        ]
        application_id = compute_authority_identity(
            AuthorityIdentityKind.RELATION_APPLICATION,
            [
                "manafold.m2.5.c.relation-application-input.v1",
                theorem_record_id.digest_bytes,
                "required_interaction",
                [member_array],
            ],
        )
        application_acceptance, application_event_binding = self._new_acceptance(
            AcceptanceSubjectKind.RELATION_APPLICATION_RECORD,
            [application_id.digest_bytes],
            "candidate-application",
            (candidate_binding,),
        )
        application_event_ref = cast(dict[str, object], application_acceptance["review_event_ref"])
        application_locator = cast(dict[str, object], application_event_ref["locator"])
        application_record_id = compute_authority_identity(
            AuthorityIdentityKind.RELATION_APPLICATION_RECORD,
            [
                "manafold.m2.5.c.relation-application-record-input.v1",
                application_id.digest_bytes,
                ReviewEventRefV1(
                    cast(str, application_event_ref["path"]),
                    bytes.fromhex(cast(str, application_event_ref["raw_sha256"])),
                    cast(str, application_locator["value"]),
                ).to_cbor(),
            ],
        )
        application = {
            "application_id": identity_wire(application_id),
            "record_id": identity_wire(application_record_id),
            "theorem_record_id": theorem["record_id"],
            "terminal_disposition": "required_interaction",
            "members": [member],
            "acceptance": application_acceptance,
        }
        base_bindings = [
            item
            for item in cast(list[dict[str, object]], self.document["source_bindings"])
            if item["artifact_role"] != "acceptance_event_leaf"
        ]
        candidate_binding_wire = self._source_binding_wire(candidate_binding, "c_candidate")
        source_bindings = [
            *base_bindings,
            self._source_binding_wire(theorem_event_binding, "acceptance_event"),
            candidate_binding_wire,
            self._source_binding_wire(application_event_binding, "acceptance_event"),
        ]
        source_bindings.sort(
            key=lambda item: encode_canonical(
                [
                    item["artifact_role"],
                    item["path"],
                    item["schema_or_null"],
                    bytes.fromhex(cast(str, item["raw_sha256"])),
                ]
            )
        )
        authority_document = {
            "schema": AUTHORITY_SCHEMA_V1,
            "model_binding": self.document["model_binding"],
            "source_bindings": source_bindings,
            "relation_proofs": [theorem],
            "relation_applications": [application],
            "domain_proofs": [],
            "domain_applications": [],
            "context_proofs": [],
            "context_applications": [],
            "supersession_records": [],
        }
        result = AuthorityValidator(resolver).validate(authority_document)
        self.assertTrue(result.valid)
        self.assertEqual(result.counts["relation_applications"], 1)

    def test_valid_same_kind_revocation_supersession_is_accepted(self) -> None:
        from authority_validator import AuthorityValidator

        theorem = cast(dict[str, object], self.document["relation_proofs"][0])
        theorem_record_id = cast(dict[str, object], theorem["record_id"])
        theorem_record_identity = AuthorityIdentityV1(
            AuthorityIdentityKind.RELATION_THEOREM_RECORD,
            bytes.fromhex(cast(str, theorem_record_id["digest_hex"])),
        )
        model_binding = self._model_binding()
        source_evidence = EvidenceRefV1(
            "model",
            MODEL_PATH,
            ("whole_artifact", None),
            model_binding.raw_sha256,
        )
        reason = SupersessionReason.AUTHORITY_REVOCATION
        acceptance, event_binding = self._new_acceptance(
            AcceptanceSubjectKind.SUPERSESSION_RECORD,
            [
                theorem_record_identity.digest_bytes,
                None,
                "relation_theorem_record",
                None,
                reason.value,
                [source_evidence.to_cbor()],
            ],
            "relation-revocation",
        )
        event_ref = cast(dict[str, object], acceptance["review_event_ref"])
        event_locator = cast(dict[str, object], event_ref["locator"])
        review_ref = ReviewEventRefV1(
            cast(str, event_ref["path"]),
            bytes.fromhex(cast(str, event_ref["raw_sha256"])),
            cast(str, event_locator["value"]),
        )
        supersession = SupersessionRecordV1(
            superseded_record_id=theorem_record_identity,
            replacement_record_id=None,
            superseded_record_kind=RecordKind.RELATION_THEOREM_RECORD,
            replacement_record_kind=None,
            reason_code=reason,
            source_evidence_refs=(source_evidence,),
            review_event_ref=review_ref,
        )
        supersession_record = {
            "supersession_id": identity_wire(supersession.identity()),
            "superseded_record_id": theorem["record_id"],
            "replacement_record_id": None,
            "superseded_record_kind": "relation_theorem_record",
            "replacement_record_kind": None,
            "reason_code": reason.value,
            "source_evidence_refs": theorem["source_evidence_refs"],
            "acceptance": acceptance,
        }
        candidate = deepcopy(self.document)
        candidate["supersession_records"] = [supersession_record]
        candidate["source_bindings"] = [
            *cast(list[dict[str, object]], self.document["source_bindings"]),
            self._source_binding_wire(event_binding, "acceptance_event"),
        ]
        cast(list[dict[str, object]], candidate["source_bindings"]).sort(
            key=lambda item: encode_canonical(
                [
                    item["artifact_role"],
                    item["path"],
                    item["schema_or_null"],
                    bytes.fromhex(cast(str, item["raw_sha256"])),
                ]
            )
        )
        result = AuthorityValidator(self.resolver).validate(candidate)
        self.assertTrue(result.valid)
        self.assertEqual(result.counts["supersession_records"], 1)


if __name__ == "__main__":
    unittest.main()
