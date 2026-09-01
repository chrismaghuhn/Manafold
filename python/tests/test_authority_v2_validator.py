from __future__ import annotations

import hashlib
import io
import json
import shutil
import sys
import tempfile
import unittest
import zipfile
from collections.abc import Mapping
from pathlib import Path
from types import MappingProxyType
from typing import cast

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from authority_source_resolver import AuthoritySourceResolver, Rev3ArchiveStore
from authority_v2_validator import (
    AuthorityV2ValidationError,
    AuthorityV2Validator,
    validate_application_host_closure,
)
from mtgml.host_binding import (
    HOST_BINDING_AUTHORITY_SCHEMA_V2,
    ApplicationHostBindingV1,
    ApplicationMemberKeyV1,
    CrossDeckHostBindingClaimV1,
    CrossDeckParticipantDiscoveryHostBindingV1,
    DiscoveryHostRefV1,
    HostBindingEvidenceRefV2,
    HostBindingSourceBindingV2,
    HostRealizationWitnessV1,
    ParticipantHostRealizationV1,
)
from mtgml.persistence import encode_canonical

MODEL_PATH = "sources/m2_5/closures/C/declared_interaction_model.v2.json"
MODEL_SCHEMA = "manafold.m2.5.c.declared-interaction-model.v2"
BASE_PATH = "sources/m2_5/authorities/interaction_review_authority.v1.json"
BASE_SCHEMA = "manafold.m2.5.c.interaction-review-authority.v1"
DECK_PATH = "inputs/deck_row_source_resolution_REV3.csv"


def _archive_bytes(members: Mapping[str, bytes]) -> bytes:
    entries = [
        {"path": path, "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}
        for path, raw in members.items()
    ]
    manifest = {
        "schema": "manafold.m2.5.rev3.package-manifest.v1",
        "manifest_excludes_self": True,
        "manifest_excluded_paths": [],
        "entries": entries,
    }
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_STORED) as archive:
        for path, raw in members.items():
            archive.writestr(path, raw)
        archive.writestr(
            "Manafold_M2_5_Package_Manifest_REV3.json",
            (json.dumps(manifest, separators=(",", ":")) + "\n").encode("utf-8"),
        )
    return output.getvalue()


def _ref(row: int) -> HostBindingEvidenceRefV2:
    return HostBindingEvidenceRefV2(
        artifact_role="rev3_card_requirement_map",
        path="derived/Card_Requirement_Map_REV3.csv",
        schema_or_null=None,
        raw_sha256=bytes(32),
        locator=("csv_row", row),
    )


def _claim(member: ApplicationMemberKeyV1, host: str, row: int) -> CrossDeckHostBindingClaimV1:
    host_ref = DiscoveryHostRefV1("rev3_deck", host)
    mapping = CrossDeckParticipantDiscoveryHostBindingV1(
        member_key=member,
        participant_position=0,
        participant_ref="cap.aura",
        discovery_side="rev3_left_family",
        discovery_host=host_ref,
        mapping_evidence_refs=(_ref(row),),
    )
    witness = HostRealizationWitnessV1(
        discovery_mapping_ref=_ref(row),
        deck_row_ref=HostBindingEvidenceRefV2(
            artifact_role="rev3_deck_row_source_resolution",
            path="inputs/deck_row_source_resolution_REV3.csv",
            schema_or_null=None,
            raw_sha256=bytes(32),
            locator=("csv_row", row),
        ),
        osi_ref=HostBindingEvidenceRefV2(
            artifact_role="rev3_osi_source_records",
            path="source/raw/oracle_cards_selected_REV3.jsonl",
            schema_or_null=None,
            raw_sha256=bytes(32),
            locator=("jsonl_line", row),
        ),
        b2_assignment_refs=(
            HostBindingEvidenceRefV2(
                artifact_role="b2_classifications",
                path="sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
                schema_or_null="manafold.m2.5.b2.card-semantic-classifications.v1",
                raw_sha256=bytes(32),
                locator=("json_pointer", "/classifications/0"),
            ),
        ),
    )
    realization = ParticipantHostRealizationV1(
        member_key=member,
        participant_position=0,
        participant_ref="cap.aura",
        host=host_ref,
        witnesses=(witness,),
    )
    return CrossDeckHostBindingClaimV1(
        member_key=member,
        discovery_bindings=(mapping,),
        participant_host_realizations=(realization,),
        observed_host_relationship="same_host",
    )


class AuthorityV2ClosureTests(unittest.TestCase):
    def setUp(self) -> None:
        self.member_a = ApplicationMemberKeyV1("candidate-a", bytes([1]) * 32, "si/a")
        self.member_b = ApplicationMemberKeyV1("candidate-b", bytes([2]) * 32, "si/b")
        self.claim_a = _claim(self.member_a, "Token Triumph", 1)
        self.claim_b = _claim(self.member_b, "Grave Danger", 2)

    def test_different_application_partitions_are_allowed(self) -> None:
        links = (
            ApplicationHostBindingV1(
                "relation_application",
                "rpa.v1/" + "01" * 32,
                (self.claim_a.identity().as_text(),),
            ),
            ApplicationHostBindingV1(
                "domain_application",
                "dpa.v1/" + "02" * 32,
                tuple(
                    sorted((self.claim_a.identity().as_text(), self.claim_b.identity().as_text()))
                ),
            ),
            ApplicationHostBindingV1(
                "context_application",
                "cpa.v1/" + "03" * 32,
                (self.claim_b.identity().as_text(),),
            ),
        )

        validate_application_host_closure(
            (self.claim_a, self.claim_b),
            links,
            MappingProxyType(
                {
                    links[0].application_semantic_id: (self.member_a,),
                    links[1].application_semantic_id: (self.member_a, self.member_b),
                    links[2].application_semantic_id: (self.member_b,),
                }
            ),
            MappingProxyType(
                {
                    links[0].application_semantic_id: "same_host",
                    links[1].application_semantic_id: "same_host",
                    links[2].application_semantic_id: "same_host",
                }
            ),
        )

    def test_missing_claim_member_fails_closed(self) -> None:
        link = ApplicationHostBindingV1(
            "domain_application",
            "dpa.v1/" + "04" * 32,
            (self.claim_a.identity().as_text(),),
        )

        with self.assertRaises(AuthorityV2ValidationError):
            validate_application_host_closure(
                (self.claim_a,),
                (link,),
                {link.application_semantic_id: (self.member_a, self.member_b)},
                {link.application_semantic_id: "same_host"},
            )

    def test_every_current_application_requires_a_host_binding_link(self) -> None:
        application_id = "rpa.v1/" + "08" * 32

        with self.assertRaises(AuthorityV2ValidationError):
            validate_application_host_closure(
                (self.claim_a,),
                (),
                {application_id: (self.member_a,)},
                {application_id: "same_host"},
            )

    def test_non_cross_deck_applications_do_not_require_host_links(self) -> None:
        host_application_id = "rpa.v1/" + "09" * 32
        non_host_application_id = "dpa.v1/" + "0a" * 32
        link = ApplicationHostBindingV1(
            "relation_application",
            host_application_id,
            (self.claim_a.identity().as_text(),),
        )

        validate_application_host_closure(
            (self.claim_a,),
            (link,),
            {
                host_application_id: (self.member_a,),
                non_host_application_id: (self.member_b,),
            },
            {host_application_id: "same_host"},
            {host_application_id},
        )

    def test_v2_does_not_replace_v1_member_order_with_member_key_order(self) -> None:
        first = ApplicationMemberKeyV1("candidate-z", bytes([1]) * 32, "si/z")
        second = ApplicationMemberKeyV1("candidate-a", bytes([2]) * 32, "si/a")
        claim_first = _claim(first, "Token Triumph", 1)
        claim_second = _claim(second, "Grave Danger", 2)
        application_id = "rpa.v1/" + "0b" * 32
        link = ApplicationHostBindingV1(
            "relation_application",
            application_id,
            tuple(sorted((claim_first.identity().as_text(), claim_second.identity().as_text()))),
        )

        validate_application_host_closure(
            (claim_first, claim_second),
            (link,),
            {application_id: (first, second)},
            {application_id: "same_host"},
            {application_id},
        )

    def test_same_member_cannot_use_two_current_claims(self) -> None:
        alternate = _claim(self.member_a, "Grave Danger", 3)
        links = (
            ApplicationHostBindingV1(
                "relation_application",
                "rpa.v1/" + "05" * 32,
                (self.claim_a.identity().as_text(),),
            ),
            ApplicationHostBindingV1(
                "domain_application",
                "dpa.v1/" + "06" * 32,
                (alternate.identity().as_text(),),
            ),
        )

        with self.assertRaises(AuthorityV2ValidationError):
            validate_application_host_closure(
                (self.claim_a, alternate),
                links,
                {
                    links[0].application_semantic_id: (self.member_a,),
                    links[1].application_semantic_id: (self.member_a,),
                },
                {
                    links[0].application_semantic_id: "same_host",
                    links[1].application_semantic_id: "same_host",
                },
            )

    def test_observed_host_relationship_must_match_theorem_expectation(self) -> None:
        link = ApplicationHostBindingV1(
            "relation_application",
            "rpa.v1/" + "07" * 32,
            (self.claim_a.identity().as_text(),),
        )

        with self.assertRaises(AuthorityV2ValidationError):
            validate_application_host_closure(
                (self.claim_a,),
                (link,),
                {link.application_semantic_id: (self.member_a,)},
                {link.application_semantic_id: "cross_host"},
            )

    def test_v1_application_facts_use_semantic_id_and_theorem_host_subject(self) -> None:
        application_id = "rpa.v1/" + "22" * 32
        document = {
            "relation_proofs": [
                {
                    "record_id": {"digest_hex": "11" * 32},
                    "subject": {"host_relationship": "cross_host"},
                }
            ],
            "relation_applications": [
                {
                    "application_id": {"digest_hex": "22" * 32},
                    "record_id": {"digest_hex": "33" * 32},
                    "theorem_record_id": {"digest_hex": "11" * 32},
                    "members": [
                        {
                            "candidate_id": "candidate",
                            "candidate_identity": {"digest_hex": "44" * 32},
                            "source_instance_id": "si.v1/candidate/0",
                        }
                    ],
                }
            ],
            "domain_proofs": [],
            "domain_applications": [],
            "context_proofs": [],
            "context_applications": [],
        }

        members, hosts, member_sources = AuthorityV2Validator._v1_application_facts(document, set())

        self.assertEqual(tuple(members), (application_id,))
        self.assertEqual(hosts[application_id], "cross_host")
        self.assertEqual(member_sources, {})


class AuthorityV2DocumentTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.repo = Path(self.tempdir.name) / "repo"
        model_path = self.repo / Path(*MODEL_PATH.split("/"))
        model_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / MODEL_PATH, model_path)
        model_raw = model_path.read_bytes()
        base_document = {
            "schema": BASE_SCHEMA,
            "model_binding": {
                "path": MODEL_PATH,
                "raw_sha256": hashlib.sha256(model_raw).hexdigest(),
                "model_id": "declared-interaction-model.v2",
                "model_version": "2",
            },
            "source_bindings": [
                {
                    "authority_kind": "model",
                    "artifact_role": "declared_model",
                    "path": MODEL_PATH,
                    "schema_or_null": MODEL_SCHEMA,
                    "raw_sha256": hashlib.sha256(model_raw).hexdigest(),
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
        base_path = self.repo / Path(*BASE_PATH.split("/"))
        base_path.parent.mkdir(parents=True, exist_ok=True)
        base_raw = (json.dumps(base_document, separators=(",", ":")) + "\n").encode("utf-8")
        base_path.write_bytes(base_raw)
        base_binding = {
            "artifact_role": "base_authority_v1",
            "path": BASE_PATH,
            "schema_or_null": BASE_SCHEMA,
            "raw_sha256": hashlib.sha256(base_raw).hexdigest(),
        }
        self.document = {
            "schema": HOST_BINDING_AUTHORITY_SCHEMA_V2,
            "base_authority_v1_binding": base_binding,
            "source_bindings": [base_binding],
            "cross_deck_host_binding_claim_records": [],
            "cross_deck_host_binding_claim_supersession_records": [],
            "application_host_bindings": [],
        }
        self.validator = AuthorityV2Validator(AuthoritySourceResolver(self.repo))

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def test_empty_v2_container_validates_its_exact_v1_base(self) -> None:
        result = self.validator.validate(self.document)

        self.assertTrue(result.valid)
        self.assertEqual(result.counts["cross_deck_host_binding_claim_records"], 0)

    def test_v1_root_cannot_be_validated_as_v2(self) -> None:
        document = dict(self.document)
        document["schema"] = "manafold.m2.5.c.interaction-review-authority.v1"

        with self.assertRaises(AuthorityV2ValidationError):
            self.validator.validate(document)

    def test_v2_root_rejects_an_unused_source_binding(self) -> None:
        document = dict(self.document)
        document["source_bindings"] = [
            *cast(list[dict[str, object]], self.document["source_bindings"]),
            {
                "artifact_role": "declared_model",
                "path": MODEL_PATH,
                "schema_or_null": MODEL_SCHEMA,
                "raw_sha256": hashlib.sha256(
                    (self.repo / Path(*MODEL_PATH.split("/"))).read_bytes()
                ).hexdigest(),
            },
        ]

        with self.assertRaises(AuthorityV2ValidationError):
            self.validator.validate(document)

    def test_v2_acceptance_event_resolves_its_bound_roster_and_identity(self) -> None:
        from mtgml.authority import (
            AcceptanceEvidenceRefV1,
            ReviewerRoleBindingV1,
            ReviewerRosterRefV1,
            ReviewMode,
        )
        from mtgml.host_binding import (
            HostBindingAcceptanceEventInputV2,
            HostBindingAcceptanceEventLeafV2,
            HostBindingAcceptanceEventRefV2,
        )

        roster_document = {
            "schema": "manafold.m2.5.c.reviewer-roster.v1",
            "reviewers": [
                {
                    "reviewer_id": "alice",
                    "roles": ["architecture_maintainer", "project_owner"],
                }
            ],
        }
        roster_raw = (json.dumps(roster_document, separators=(",", ":")) + "\n").encode("utf-8")
        roster_digest = hashlib.sha256(roster_raw).digest()
        roster_path = (
            "sources/m2_5/authorities/reviewer_rosters/v1/" + roster_digest.hex() + ".json"
        )
        roster_file = self.repo / Path(*roster_path.split("/"))
        roster_file.parent.mkdir(parents=True, exist_ok=True)
        roster_file.write_bytes(roster_raw)

        evidence_path = self.repo / "docs" / "review" / "host-binding.md"
        evidence_path.parent.mkdir(parents=True, exist_ok=True)
        evidence_raw = b"host binding review evidence\n"
        evidence_path.write_bytes(evidence_raw)

        model_file = self.repo / Path(*MODEL_PATH.split("/"))
        model_digest = hashlib.sha256(model_file.read_bytes()).digest()
        model_binding = HostBindingSourceBindingV2(
            "declared_model",
            MODEL_PATH,
            MODEL_SCHEMA,
            model_digest,
        )
        roster_binding = HostBindingSourceBindingV2(
            "reviewer_roster_leaf",
            roster_path,
            "manafold.m2.5.c.reviewer-roster.v1",
            roster_digest,
        )
        event_input = HostBindingAcceptanceEventInputV2(
            subject_kind="cross_deck_host_binding_claim_record_v1",
            subject_payload_digest=bytes(32),
            reviewer_roster_ref=ReviewerRosterRefV1(
                roster_path,
                "manafold.m2.5.c.reviewer-roster.v1",
                roster_digest,
            ),
            reviewer_role_bindings=(
                ReviewerRoleBindingV1("alice", ("architecture_maintainer", "project_owner")),
            ),
            review_mode=ReviewMode.SOLO_SEPARATE_SELF_REVIEW,
            checklist_id="cross-deck-host-binding-review-checklist.v1",
            source_binding_digests=(model_binding, roster_binding),
            review_evidence_refs=(
                AcceptanceEvidenceRefV1(
                    "docs/review/host-binding.md",
                    hashlib.sha256(evidence_raw).digest(),
                    ("whole_artifact", None),
                ),
            ),
        )
        leaf = HostBindingAcceptanceEventLeafV2.from_input(event_input)
        event_raw = (json.dumps(leaf.to_wire(), separators=(",", ":")) + "\n").encode("utf-8")
        event_path = (
            "sources/m2_5/authorities/review_acceptance_events/v2/"
            + leaf.event_id.as_text().removeprefix("ae.v2/")
            + ".json"
        )
        event_file = self.repo / Path(*event_path.split("/"))
        event_file.parent.mkdir(parents=True, exist_ok=True)
        event_file.write_bytes(event_raw)
        event_ref = HostBindingAcceptanceEventRefV2(
            event_path,
            hashlib.sha256(event_raw).digest(),
            leaf.event_id.as_text(),
        )

        resolved = self.validator._validate_acceptance_event(
            event_ref,
            "cross_deck_host_binding_claim_record_v1",
            bytes(32),
        )

        self.assertEqual(resolved.identity(), event_input.identity())

    def test_v2_acceptance_requires_architecture_maintainer(self) -> None:
        from mtgml.authority import (
            AcceptanceEvidenceRefV1,
            ReviewerRoleBindingV1,
            ReviewerRosterRefV1,
            ReviewMode,
        )
        from mtgml.host_binding import (
            HostBindingAcceptanceEventInputV2,
            HostBindingAcceptanceEventLeafV2,
            HostBindingAcceptanceEventRefV2,
        )

        roster_document = {
            "schema": "manafold.m2.5.c.reviewer-roster.v1",
            "reviewers": [{"reviewer_id": "alice", "roles": ["project_owner"]}],
        }
        roster_raw = (json.dumps(roster_document, separators=(",", ":")) + "\n").encode("utf-8")
        roster_digest = hashlib.sha256(roster_raw).digest()
        roster_path = (
            "sources/m2_5/authorities/reviewer_rosters/v1/" + roster_digest.hex() + ".json"
        )
        roster_file = self.repo / Path(*roster_path.split("/"))
        roster_file.parent.mkdir(parents=True, exist_ok=True)
        roster_file.write_bytes(roster_raw)

        evidence_path = self.repo / "docs" / "review" / "host-binding-role.md"
        evidence_path.parent.mkdir(parents=True, exist_ok=True)
        evidence_raw = b"host binding role evidence\n"
        evidence_path.write_bytes(evidence_raw)

        model_file = self.repo / Path(*MODEL_PATH.split("/"))
        model_binding = HostBindingSourceBindingV2(
            "declared_model",
            MODEL_PATH,
            MODEL_SCHEMA,
            hashlib.sha256(model_file.read_bytes()).digest(),
        )
        roster_binding = HostBindingSourceBindingV2(
            "reviewer_roster_leaf",
            roster_path,
            "manafold.m2.5.c.reviewer-roster.v1",
            roster_digest,
        )
        event_input = HostBindingAcceptanceEventInputV2(
            "cross_deck_host_binding_claim_record_v1",
            bytes(32),
            ReviewerRosterRefV1(roster_path, roster_binding.schema_or_null, roster_digest),
            (ReviewerRoleBindingV1("alice", ("project_owner",)),),
            ReviewMode.SOLO_SEPARATE_SELF_REVIEW,
            "cross-deck-host-binding-review-checklist.v1",
            (model_binding, roster_binding),
            (
                AcceptanceEvidenceRefV1(
                    "docs/review/host-binding-role.md",
                    hashlib.sha256(evidence_raw).digest(),
                    ("whole_artifact", None),
                ),
            ),
        )
        leaf = HostBindingAcceptanceEventLeafV2.from_input(event_input)
        event_raw = (json.dumps(leaf.to_wire(), separators=(",", ":")) + "\n").encode("utf-8")
        event_path = (
            "sources/m2_5/authorities/review_acceptance_events/v2/"
            + leaf.event_id.as_text().removeprefix("ae.v2/")
            + ".json"
        )
        event_file = self.repo / Path(*event_path.split("/"))
        event_file.parent.mkdir(parents=True, exist_ok=True)
        event_file.write_bytes(event_raw)

        with self.assertRaises(AuthorityV2ValidationError):
            self.validator._validate_acceptance_event(
                HostBindingAcceptanceEventRefV2(
                    event_path,
                    hashlib.sha256(event_raw).digest(),
                    leaf.event_id.as_text(),
                ),
                "cross_deck_host_binding_claim_record_v1",
                bytes(32),
            )

    def test_v2_claim_root_revalidates_source_event_and_exact_source_closure(self) -> None:
        from mtgml.authority import (
            AcceptanceEvidenceRefV1,
            ReviewerRoleBindingV1,
            ReviewerRosterRefV1,
            ReviewMode,
        )
        from mtgml.host_binding import (
            ApplicationMemberKeyV1,
            CrossDeckHostBindingClaimRecordV1,
            CrossDeckHostBindingClaimV1,
            CrossDeckParticipantDiscoveryHostBindingV1,
            DiscoveryHostRefV1,
            HostBindingAcceptanceEventInputV2,
            HostBindingAcceptanceEventLeafV2,
            HostBindingAcceptanceEventRefV2,
            HostBindingEvidenceRefV2,
            HostRealizationWitnessV1,
            ParticipantHostRealizationV1,
        )

        map_path = "derived/Card_Requirement_Map_REV3.csv"
        deck_path = DECK_PATH
        osi_path = "source/raw/oracle_cards_selected_REV3.jsonl"
        b2_path = "sources/m2_5/closures/B2/card_semantic_classifications.v1.json"
        b2_catalog_path = "sources/m2_5/closures/B2/requirement_family_catalog.v1.json"
        b2_closure_path = "sources/m2_5/closures/B2/classification_closure.v1.json"
        b2_classification_document = cast(
            dict[str, object],
            json.loads((ROOT / Path(*b2_path.split("/"))).read_text(encoding="utf-8")),
        )
        b2_classifications = cast(
            list[dict[str, object]], b2_classification_document["classifications"]
        )
        first_classification = b2_classifications[0]
        oracle_id = cast(str, first_classification["oracle_semantic_identity"])
        family_id = cast(
            str,
            cast(list[dict[str, object]], first_classification["requirement_assignments"])[0][
                "requirement_family_id"
            ],
        )
        map_raw = (
            b"deck_row_id,deck_id,oracle_semantic_identity,requirement_id,provenance,ranking_eligible\n"
            + (
                f"token:line-1,Token Triumph,{oracle_id},{family_id},"
                "INHERITED_REV2_CANDIDATE,False\n"
            ).encode()
        )
        deck_raw = (
            b"deck_row_id,deck_id,oracle_semantic_identity,card,quantity,oracle_top_level_text,oracle_faces\n"
            + f"token:line-1,Token Triumph,{oracle_id},Token Aura,1,,\n".encode()
        )
        osi_raw = (
            json.dumps({"oracle_id": oracle_id, "name": "Token Aura"}, separators=(",", ":")) + "\n"
        ).encode("utf-8")
        archive_raw = _archive_bytes({map_path: map_raw, deck_path: deck_raw, osi_path: osi_raw})
        archive = Rev3ArchiveStore.from_bytes(
            archive_raw,
            hashlib.sha256(archive_raw).hexdigest(),
        )
        b2_file = self.repo / Path(*b2_path.split("/"))
        b2_file.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / Path(*b2_path.split("/")), b2_file)
        b2_raw = b2_file.read_bytes()
        b2_file.write_bytes(b2_raw)
        for b2_artifact_path in (b2_catalog_path, b2_closure_path):
            target = self.repo / Path(*b2_artifact_path.split("/"))
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / Path(*b2_artifact_path.split("/")), target)

        member = ApplicationMemberKeyV1("candidate", bytes(32), "si.v1/candidate/0")
        map_ref = HostBindingEvidenceRefV2(
            "rev3_card_requirement_map",
            map_path,
            None,
            hashlib.sha256(map_raw).digest(),
            ("csv_row", 0),
        )
        osi_ref = HostBindingEvidenceRefV2(
            "rev3_osi_source_records",
            osi_path,
            None,
            hashlib.sha256(osi_raw).digest(),
            ("jsonl_line", 0),
        )
        b2_ref = HostBindingEvidenceRefV2(
            "b2_classifications",
            b2_path,
            "manafold.m2.5.b2.card-semantic-classifications.v1",
            hashlib.sha256(b2_raw).digest(),
            ("json_pointer", "/classifications/0"),
        )
        host = DiscoveryHostRefV1("rev3_deck", "Token Triumph")
        discovery = CrossDeckParticipantDiscoveryHostBindingV1(
            member,
            0,
            family_id,
            "rev3_left_family",
            host,
            (map_ref,),
        )
        deck_ref = HostBindingEvidenceRefV2(
            "rev3_deck_row_source_resolution",
            deck_path,
            None,
            hashlib.sha256(deck_raw).digest(),
            ("csv_row", 0),
        )
        witness = HostRealizationWitnessV1(map_ref, deck_ref, osi_ref, (b2_ref,))
        realization = ParticipantHostRealizationV1(member, 0, family_id, host, (witness,))
        claim = CrossDeckHostBindingClaimV1(member, (discovery,), (realization,), "same_host")

        roster_document = {
            "schema": "manafold.m2.5.c.reviewer-roster.v1",
            "reviewers": [
                {
                    "reviewer_id": "alice",
                    "roles": ["architecture_maintainer", "project_owner"],
                }
            ],
        }
        roster_raw = (json.dumps(roster_document, separators=(",", ":")) + "\n").encode("utf-8")
        roster_digest = hashlib.sha256(roster_raw).digest()
        roster_path = (
            "sources/m2_5/authorities/reviewer_rosters/v1/" + roster_digest.hex() + ".json"
        )
        roster_file = self.repo / Path(*roster_path.split("/"))
        roster_file.parent.mkdir(parents=True, exist_ok=True)
        roster_file.write_bytes(roster_raw)
        evidence_path = self.repo / "docs" / "review" / "host-binding.md"
        evidence_path.parent.mkdir(parents=True, exist_ok=True)
        evidence_raw = b"host binding acceptance evidence\n"
        evidence_path.write_bytes(evidence_raw)

        model_file = self.repo / Path(*MODEL_PATH.split("/"))
        model_binding = HostBindingSourceBindingV2(
            "declared_model",
            MODEL_PATH,
            MODEL_SCHEMA,
            hashlib.sha256(model_file.read_bytes()).digest(),
        )
        roster_binding = HostBindingSourceBindingV2(
            "reviewer_roster_leaf",
            roster_path,
            "manafold.m2.5.c.reviewer-roster.v1",
            roster_digest,
        )
        event_sources = tuple(
            sorted(
                (
                    model_binding,
                    roster_binding,
                    HostBindingSourceBindingV2(
                        "rev3_card_requirement_map", map_path, None, map_ref.raw_sha256
                    ),
                    HostBindingSourceBindingV2(
                        "rev3_deck_row_source_resolution",
                        deck_path,
                        None,
                        deck_ref.raw_sha256,
                    ),
                    HostBindingSourceBindingV2(
                        "rev3_osi_source_records", osi_path, None, osi_ref.raw_sha256
                    ),
                    HostBindingSourceBindingV2(
                        "b2_classifications",
                        b2_path,
                        b2_ref.schema_or_null,
                        b2_ref.raw_sha256,
                    ),
                    HostBindingSourceBindingV2(
                        "b2_catalog",
                        b2_catalog_path,
                        "manafold.m2.5.b2.requirement-family-catalog.v1",
                        hashlib.sha256(
                            (self.repo / Path(*b2_catalog_path.split("/"))).read_bytes()
                        ).digest(),
                    ),
                    HostBindingSourceBindingV2(
                        "b2_closure",
                        b2_closure_path,
                        "manafold.m2.5.b2.classification-closure.v1",
                        hashlib.sha256(
                            (self.repo / Path(*b2_closure_path.split("/"))).read_bytes()
                        ).digest(),
                    ),
                ),
                key=lambda binding: encode_canonical(binding.to_cbor()),
            )
        )
        event_input = HostBindingAcceptanceEventInputV2(
            "cross_deck_host_binding_claim_record_v1",
            claim.identity().digest_bytes,
            ReviewerRosterRefV1(roster_path, roster_binding.schema_or_null, roster_digest),
            (ReviewerRoleBindingV1("alice", ("architecture_maintainer", "project_owner")),),
            ReviewMode.SOLO_SEPARATE_SELF_REVIEW,
            "cross-deck-host-binding-review-checklist.v1",
            event_sources,
            (
                AcceptanceEvidenceRefV1(
                    "docs/review/host-binding.md",
                    hashlib.sha256(evidence_raw).digest(),
                    ("whole_artifact", None),
                ),
            ),
        )
        leaf = HostBindingAcceptanceEventLeafV2.from_input(event_input)
        event_raw = (json.dumps(leaf.to_wire(), separators=(",", ":")) + "\n").encode("utf-8")
        event_path = (
            "sources/m2_5/authorities/review_acceptance_events/v2/"
            + leaf.event_id.as_text().removeprefix("ae.v2/")
            + ".json"
        )
        event_file = self.repo / Path(*event_path.split("/"))
        event_file.parent.mkdir(parents=True, exist_ok=True)
        event_file.write_bytes(event_raw)
        record = CrossDeckHostBindingClaimRecordV1(
            claim,
            HostBindingAcceptanceEventRefV2(
                event_path,
                hashlib.sha256(event_raw).digest(),
                leaf.event_id.as_text(),
            ),
        )
        base_binding = HostBindingSourceBindingV2(
            "base_authority_v1",
            BASE_PATH,
            BASE_SCHEMA,
            hashlib.sha256((self.repo / Path(*BASE_PATH.split("/"))).read_bytes()).digest(),
        )
        event_binding = HostBindingSourceBindingV2(
            "acceptance_event_leaf_v2",
            event_path,
            "manafold.m2.5.c.review-acceptance-event.v2",
            hashlib.sha256(event_raw).digest(),
        )
        root_sources = tuple(
            sorted(
                (base_binding, event_binding, *event_sources),
                key=lambda binding: encode_canonical(binding.to_cbor()),
            )
        )
        document = {
            "schema": HOST_BINDING_AUTHORITY_SCHEMA_V2,
            "base_authority_v1_binding": base_binding.to_wire(),
            "source_bindings": [binding.to_wire() for binding in root_sources],
            "cross_deck_host_binding_claim_records": [record.to_wire()],
            "cross_deck_host_binding_claim_supersession_records": [],
            "application_host_bindings": [],
        }
        validator = AuthorityV2Validator(AuthoritySourceResolver(self.repo, rev3_archive=archive))

        result = validator.validate(document)

        self.assertTrue(result.valid)
        self.assertEqual(result.counts["cross_deck_host_binding_claim_records"], 1)


if __name__ == "__main__":
    unittest.main()
