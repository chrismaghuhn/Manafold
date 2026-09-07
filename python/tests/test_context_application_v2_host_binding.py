from __future__ import annotations

import base64
import hashlib
import json
import sys
import unittest
from dataclasses import FrozenInstanceError, replace
from pathlib import Path
from types import SimpleNamespace
from typing import cast
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from authority_host_binding import HostBindingSourceError
from authority_source_resolver import (
    AuthoritySourceResolver,
    ResolutionError,
    ResolutionStatus,
    Rev3ArchiveStore,
)
from authority_v2_validator import (
    AuthorityV2Validator,
    HostBindingAuthorityV2ReadModel,
    HostBindingClaimRecordStatus,
)
from context_application_v2_host_binding import (
    ContextApplicationV2HostBindingError,
    ContextApplicationV2HostBindingEvaluationResult,
    ContextApplicationV2HostBindingEvaluator,
    _ResolvedApplicationMember,
    _validate_link_member_union,
)
from context_application_v2_resolver import (
    ContextApplicationV2Resolver,
    canonical_source_bindings,
)
from context_application_v2_test_support import (
    build_application_record_with_v3_event,
    build_application_variant_with_v3_event,
    build_application_with_v3_event,
    build_supersession_with_v3_event,
)
from mtgml.authority import (
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKind,
    AcceptanceSubjectPayloadV1,
    ApplicationHostBindingV2,
    AuthorityContractError,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationAuthorityV2,
    ContextApplicationV2InputV1,
    ContextApplicationV2Record,
    ContextApplicationV2SupersessionInputV2,
    ContextApplicationV2SupersessionRecord,
    ContextAuthoritySourceBindingV2,
    DigestReferenceV1,
    ReviewAcceptanceEventInputV1,
    ReviewAcceptanceEventLeafV1,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewEventRefV1,
    ReviewEventRefV3,
    ReviewMode,
    SourceBindingDigestV1,
    SupersessionReason,
    compute_authority_identity,
)
from mtgml.host_binding import (
    ApplicationMemberKeyV1,
    CrossDeckHostBindingClaimRecordV1,
    CrossDeckHostBindingClaimSupersessionV1,
    CrossDeckHostBindingClaimV1,
    CrossDeckParticipantDiscoveryHostBindingV1,
    DiscoveryHostRefV1,
    HostBindingAcceptanceEventInputV2,
    HostBindingAcceptanceEventLeafV2,
    HostBindingAcceptanceEventRefV2,
    HostBindingEvidenceRefV2,
    HostBindingSourceBindingV2,
    HostRealizationWitnessV1,
    ParticipantHostRealizationV1,
)
from mtgml.persistence import encode_canonical
from test_authority_v2_validator import _claim

BASE_BINDING = ContextAuthoritySourceBindingV2(
    "base_authority_v1",
    "sources/m2_5/authorities/interaction_review_authority.v1.json",
    "manafold.m2.5.c.interaction-review-authority.v1",
    bytes([1]) * 32,
)
CANDIDATE_BINDING = ContextAuthoritySourceBindingV2(
    "candidate_universe",
    "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
    "manafold.m2.5.c.interaction-candidate-universe.v2",
    bytes([2]) * 32,
)
HOST_BINDING = ContextAuthoritySourceBindingV2(
    "host_binding_authority_v2",
    "sources/m2_5/authorities/interaction_review_authority.v2.json",
    "manafold.m2.5.c.interaction-review-authority.v2",
    bytes([3]) * 32,
)


def _application_id(marker: int) -> AuthorityIdentityV1:
    return AuthorityIdentityV1(
        AuthorityIdentityKind.CONTEXT_APPLICATION_V2,
        bytes([marker]) * 32,
    )


def _claim_id(marker: int) -> str:
    return "hbc.v1/" + f"{marker:02x}" * 32


def _link(marker: int, claim_marker: int) -> ApplicationHostBindingV2:
    return ApplicationHostBindingV2(
        "context_application",
        _application_id(marker),
        (_claim_id(claim_marker),),
    )


def _cross_host_claim(member_key: ApplicationMemberKeyV1) -> CrossDeckHostBindingClaimV1:
    discovery_bindings = []
    realizations = []
    for position, (participant_ref, host_id) in enumerate(
        (("cap.aura", "Token Triumph"), ("cap.draw", "Grave Danger"))
    ):
        mapping_ref = HostBindingEvidenceRefV2(
            "rev3_card_requirement_map",
            "derived/Card_Requirement_Map_REV3.csv",
            None,
            bytes(32),
            ("csv_row", position),
        )
        deck_ref = HostBindingEvidenceRefV2(
            "rev3_deck_row_source_resolution",
            "inputs/deck_row_source_resolution_REV3.csv",
            None,
            bytes(32),
            ("csv_row", position),
        )
        osi_ref = HostBindingEvidenceRefV2(
            "rev3_osi_source_records",
            "source/raw/oracle_cards_selected_REV3.jsonl",
            None,
            bytes(32),
            ("jsonl_line", position),
        )
        b2_ref = HostBindingEvidenceRefV2(
            "b2_classifications",
            "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
            "manafold.m2.5.b2.card-semantic-classifications.v1",
            bytes(32),
            ("json_pointer", f"/classifications/{position}"),
        )
        host = DiscoveryHostRefV1("rev3_deck", host_id)
        discovery = CrossDeckParticipantDiscoveryHostBindingV1(
            member_key,
            position,
            participant_ref,
            "rev3_left_family" if position == 0 else "rev3_right_family",
            host,
            (mapping_ref,),
        )
        realization = ParticipantHostRealizationV1(
            member_key,
            position,
            participant_ref,
            host,
            (HostRealizationWitnessV1(mapping_ref, deck_ref, osi_ref, (b2_ref,)),),
        )
        discovery_bindings.append(discovery)
        realizations.append(realization)
    return CrossDeckHostBindingClaimV1(
        member_key,
        tuple(discovery_bindings),
        tuple(realizations),
        "cross_host",
    )


def _empty_container(
    *,
    links: tuple[ApplicationHostBindingV2, ...] = (),
    host_binding: ContextAuthoritySourceBindingV2 | None = None,
) -> ContextApplicationAuthorityV2:
    return ContextApplicationAuthorityV2(
        base_authority_v1_binding=BASE_BINDING,
        host_binding_authority_v2_binding=host_binding,
        candidate_universe_binding=CANDIDATE_BINDING,
        source_bindings=(),
        context_application_v2_records=(),
        context_application_v2_supersession_records=(),
        application_host_bindings_v2=links,
    )


def _file_digests(repo: Path) -> dict[str, str]:
    return {
        path.relative_to(repo).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in repo.rglob("*")
        if path.is_file()
    }


class ContextApplicationV2HostBindingEvaluatorTests(unittest.TestCase):
    def _synthetic_case(self, *, two_candidates: bool = False) -> dict[str, object]:
        from test_context_application_v2_validator import ContextApplicationV2IntegrationTests

        base = ContextApplicationV2IntegrationTests()
        base.setUp()
        self.addCleanup(base.doCleanups)
        self.addCleanup(base.tearDown)
        return base._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
            two_candidates=two_candidates,
        )

    def _mixed_application_case(
        self,
    ) -> tuple[
        dict[str, object],
        AuthoritySourceResolver,
        ContextApplicationV2Record,
        object,
        object,
        ContextAuthoritySourceBindingV2,
    ]:
        from test_authority_source_resolver import (
            REV3_CENSUS_MEMBER,
            REV3_SOURCE_COLUMNS,
            archive_bytes,
            digest,
            json_bytes,
        )

        case = self._synthetic_case(two_candidates=True)
        fixture = case["fixture"]
        universe_path = fixture.repo / Path(
            *["sources", "m2_5", "closures", "C", "interaction_candidate_universe.v2.json"]
        )
        universe = cast(dict[str, object], json.loads(universe_path.read_text(encoding="utf-8")))
        candidates = cast(list[dict[str, object]], universe["candidates"])
        instances = cast(list[dict[str, object]], universe["source_instances"])

        intra_candidate_id = "INTRA_DECK|P2|family.a|family.b|DIRECTIONAL_BINARY"
        intra_source_instance_id = (
            "si.v1/"
            + base64.urlsafe_b64encode(intra_candidate_id.encode("utf-8"))
            .decode("ascii")
            .rstrip("=")
            + "/0"
        )
        rows = [
            fixture._source_values(),
            [
                intra_candidate_id,
                "interaction-model.v1",
                "INTRA_DECK",
                "P2",
                "family.a",
                "family.b",
                "UNORDERED_BINARY",
                "AMBIGUOUS_REQUIRES_REVIEW",
                "candidate classification authority or interaction "
                "trigger is not terminally reviewed",
                '["family.a", "family.b"]',
            ],
        ]
        census_raw = fixture._census_bytes(rows)
        archive_raw = archive_bytes({REV3_CENSUS_MEMBER: census_raw})
        archive = Rev3ArchiveStore.from_bytes(archive_raw, digest(archive_raw))
        census_digest = digest(census_raw)
        archive_digest = digest(archive_raw)

        def source_binding(row_ordinal: int) -> dict[str, object]:
            return {
                "kind": "rev3",
                "archive_member": REV3_CENSUS_MEMBER,
                "archive_member_sha256": census_digest,
                "row_ordinal": row_ordinal,
                "source_columns": list(REV3_SOURCE_COLUMNS),
                "source_values": rows[row_ordinal],
            }

        for index, candidate in enumerate(candidates):
            if index == 1:
                candidate["candidate_id"] = intra_candidate_id
                candidate["scope"] = "intra_deck"
                candidate["relation"] = "unordered_binary"
            candidate["source_binding"] = source_binding(index)
            candidate["candidate_identity"] = fixture._candidate_identity_for_test(candidate)

        instances[0]["source_binding"] = source_binding(0)
        instances[1]["candidate_id"] = intra_candidate_id
        instances[1]["source_instance_id"] = intra_source_instance_id
        instances[1]["source_binding"] = source_binding(1)

        input_bindings = cast(dict[str, object], universe["input_bindings"])
        rev3_input = cast(dict[str, object], input_bindings["rev3_candidate_source"])
        rev3_input["archive_member_sha256"] = census_digest
        rev3_input["source_package_sha256"] = archive_digest
        universe_raw = json_bytes(universe)
        fixture.write_repo(
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            universe_raw,
        )
        candidate_universe_digest = bytes.fromhex(digest(universe_raw))
        source_resolver = AuthoritySourceResolver(fixture.repo, rev3_archive=archive)
        case["source_resolver"] = source_resolver

        base_record = cast(ContextApplicationV2Record, case["record"])
        base_member = base_record.members[0]

        def digest_reference(identity: dict[str, str]) -> DigestReferenceV1:
            return DigestReferenceV1(
                identity["envelope_id"],
                identity["algorithm_id"],
                identity["semantic_domain"],
                identity["payload_codec_id"],
                identity["input_schema_id"],
                bytes.fromhex(identity["digest_hex"]),
            )

        member_a = replace(
            base_member,
            candidate_identity_digest_reference=digest_reference(
                candidates[0]["candidate_identity"]
            ),
            candidate_universe_binding=[
                "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
                "manafold.m2.5.c.interaction-candidate-universe.v2",
                candidate_universe_digest,
            ],
        )
        member_b = replace(
            member_a,
            candidate_id=intra_candidate_id,
            candidate_identity_digest_reference=digest_reference(
                candidates[1]["candidate_identity"]
            ),
            source_instance_id=intra_source_instance_id,
            context_binding_v1=[
                "binary",
                "symmetric",
                member_a.context_binding_v1[2],
                "same_host",
            ],
        )
        members = tuple(
            sorted(
                (member_a, member_b),
                key=lambda member: encode_canonical(
                    [
                        member.candidate_identity_digest_reference.digest_bytes,
                        member.source_instance_id,
                    ]
                ),
            )
        )
        zero_ref = ReviewEventRefV3(
            "sources/m2_5/authorities/review_acceptance_events/v3/" + "00" * 32 + ".json",
            bytes(32),
            "ae.v3/" + "00" * 32,
        )
        mixed_application_id = ContextApplicationV2InputV1(
            theorem_record_id_bytes=base_record.theorem_record_id.digest_bytes,
            members=members,
        ).identity()
        provisional = ContextApplicationV2Record.from_parts(
            application_id=mixed_application_id,
            theorem_record_id=base_record.theorem_record_id,
            members=members,
            review_event_ref_v3=zero_ref,
        )
        candidate_binding = ContextAuthoritySourceBindingV2(
            "candidate_universe",
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            "manafold.m2.5.c.interaction-candidate-universe.v2",
            candidate_universe_digest,
        )
        return case, source_resolver, provisional, member_a, member_b, candidate_binding

    def _full_linked_case(
        self,
    ) -> tuple[
        dict[str, object],
        AuthoritySourceResolver,
        ContextApplicationV2Record,
        CrossDeckHostBindingClaimV1,
        ContextAuthoritySourceBindingV2,
        tuple[ContextAuthoritySourceBindingV2, ...],
    ]:
        """Build one fully linked context/host fixture through real admission paths."""

        from test_authority_source_resolver import (
            REV3_CENSUS_MEMBER,
            REV3_SOURCE_COLUMNS,
            archive_bytes,
            digest,
            json_bytes,
        )

        case = self._synthetic_case()
        fixture = case["fixture"]
        b2_classifications_path = "sources/m2_5/closures/B2/card_semantic_classifications.v1.json"
        b2_document = cast(
            dict[str, object],
            json.loads(
                (ROOT / Path(*b2_classifications_path.split("/"))).read_text(encoding="utf-8")
            ),
        )
        first_classification = cast(list[dict[str, object]], b2_document["classifications"])[0]
        assignments = cast(list[dict[str, object]], first_classification["requirement_assignments"])
        family_a = cast(str, assignments[0]["requirement_family_id"])
        family_b = cast(str, assignments[1]["requirement_family_id"])
        oracle_id = cast(str, first_classification["oracle_semantic_identity"])

        # Rebind the synthetic candidate/source row to real B2 family IDs.
        universe_path = fixture.repo / Path(
            *["sources", "m2_5", "closures", "C", "interaction_candidate_universe.v2.json"]
        )
        universe = cast(dict[str, object], json.loads(universe_path.read_text(encoding="utf-8")))
        candidates = cast(list[dict[str, object]], universe["candidates"])
        instances = cast(list[dict[str, object]], universe["source_instances"])
        candidate_id = f"CROSS_DECK|P1|{family_a}|{family_b}|DIRECTIONAL_BINARY"
        source_instance_id = (
            "si.v1/"
            + base64.urlsafe_b64encode(candidate_id.encode("utf-8")).decode("ascii").rstrip("=")
            + "/0"
        )
        census_row = [
            candidate_id,
            "interaction-model.v1",
            "CROSS_DECK",
            "P1",
            family_a,
            family_b,
            "DIRECTIONAL_BINARY",
            "AMBIGUOUS_REQUIRES_REVIEW",
            "synthetic linked host-binding candidate",
            json.dumps([family_a, family_b], separators=(",", ":")),
        ]
        map_path = "derived/Card_Requirement_Map_REV3.csv"
        deck_path = "inputs/deck_row_source_resolution_REV3.csv"
        osi_path = "source/raw/oracle_cards_selected_REV3.jsonl"
        pair_path = "derived/Pair_Requirement_Aggregates_REV3.json"
        map_raw = (
            b"deck_row_id,deck_id,oracle_semantic_identity,requirement_id,provenance,ranking_eligible\n"
            + (
                f"token:line-1,Token Triumph,{oracle_id},{family_a},"
                "INHERITED_REV2_CANDIDATE,False\n"
            ).encode()
            + (
                f"grave:line-2,Grave Danger,{oracle_id},{family_b},INHERITED_REV2_CANDIDATE,False\n"
            ).encode()
        )
        deck_raw = (
            b"deck_row_id,deck_id,oracle_semantic_identity,card,quantity,oracle_top_level_text,oracle_faces\n"
            + f"token:line-1,Token Triumph,{oracle_id},Token Aura,1,,\n".encode()
            + f"grave:line-2,Grave Danger,{oracle_id},Grave Draw,1,,\n".encode()
        )
        osi_raw = (
            json.dumps({"oracle_id": oracle_id, "name": "Token Aura"}, separators=(",", ":"))
            + "\n"
            + json.dumps({"oracle_id": oracle_id, "name": "Grave Draw"}, separators=(",", ":"))
            + "\n"
        ).encode("utf-8")
        pair_raw = b'{"pairs":{"P1":{"pair":["Token Triumph","Grave Danger"]}}}\n'
        census_raw = fixture._census_bytes([census_row])
        archive_raw = archive_bytes(
            {
                REV3_CENSUS_MEMBER: census_raw,
                map_path: map_raw,
                deck_path: deck_raw,
                osi_path: osi_raw,
                pair_path: pair_raw,
            }
        )
        archive = Rev3ArchiveStore.from_bytes(archive_raw, digest(archive_raw))
        census_digest = digest(census_raw)
        archive_digest = digest(archive_raw)
        source_binding = {
            "kind": "rev3",
            "archive_member": REV3_CENSUS_MEMBER,
            "archive_member_sha256": census_digest,
            "row_ordinal": 0,
            "source_columns": list(REV3_SOURCE_COLUMNS),
            "source_values": census_row,
        }
        candidate = candidates[0]
        candidate["candidate_id"] = candidate_id
        candidate["participant_refs"] = [
            {"participant_kind": "requirement_family", "semantic_ref": family_a},
            {"participant_kind": "requirement_family", "semantic_ref": family_b},
        ]
        candidate["supporting_requirement_ids"] = [family_a, family_b]
        candidate["source_binding"] = source_binding
        candidate["candidate_identity"] = fixture._candidate_identity_for_test(candidate)
        instance = instances[0]
        instance["candidate_id"] = candidate_id
        instance["source_instance_id"] = source_instance_id
        instance["source_binding"] = source_binding
        instance["participant_bindings"] = [
            {
                "role": "ordered_participant",
                "participant_ref": {
                    "participant_kind": "requirement_family",
                    "semantic_ref": family_a,
                },
            },
            {
                "role": "ordered_participant",
                "participant_ref": {
                    "participant_kind": "requirement_family",
                    "semantic_ref": family_b,
                },
            },
        ]
        input_bindings = cast(dict[str, object], universe["input_bindings"])
        rev3_input = cast(dict[str, object], input_bindings["rev3_candidate_source"])
        rev3_input["archive_member_sha256"] = census_digest
        rev3_input["source_package_sha256"] = archive_digest
        universe_raw = json_bytes(universe)
        fixture.write_repo(
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            universe_raw,
        )
        candidate_universe_digest = bytes.fromhex(digest(universe_raw))
        source_resolver = AuthoritySourceResolver(fixture.repo, rev3_archive=archive)
        case["source_resolver"] = source_resolver

        # Rebind the V1 context theorem to the same real participant families.
        base_path = cast(ContextAuthoritySourceBindingV2, case["base_binding"]).path
        base_file = fixture.repo / Path(*base_path.split("/"))
        base_document = cast(dict[str, object], json.loads(base_file.read_text(encoding="utf-8")))
        theorem = cast(list[dict[str, object]], base_document["context_proofs"])[0]
        subject_shape = {
            **cast(dict[str, object], theorem["subject_shape"]),
            "participant_roles": [
                {
                    "position": 0,
                    "role": "ordered_participant",
                    "participant_kind": "requirement_family",
                    "semantic_ref": family_a,
                },
                {
                    "position": 1,
                    "role": "ordered_participant",
                    "participant_kind": "requirement_family",
                    "semantic_ref": family_b,
                },
            ],
        }
        subject_shape_cbor = [
            subject_shape["arity"],
            subject_shape["directionality"],
            [
                [0, "ordered_participant", "requirement_family", family_a],
                [1, "ordered_participant", "requirement_family", family_b],
            ],
            subject_shape["host_relationship"],
        ]
        precondition_arrays = [
            [
                item["precondition_id"],
                [item["precondition_kind"], item["payload"]],
            ]
            for item in cast(list[dict[str, object]], theorem["preconditions"])
        ]
        theorem_id = compute_authority_identity(
            AuthorityIdentityKind.CONTEXT_THEOREM,
            [
                "manafold.m2.5.c.context-proof-input.v1",
                "declared-interaction-model.v2",
                subject_shape_cbor,
                theorem["context_dimensions"],
                theorem["temporal_semantics"],
                precondition_arrays,
                [],
                [],
            ],
        )
        source_evidence = (
            cast(ContextApplicationV2Record, case["record"]).members[0].member_evidence_refs[0]
        )
        rationale = cast(str, theorem["semantic_rationale"])
        old_event_ref = cast(
            dict[str, object], cast(dict[str, object], theorem["acceptance"])["review_event_ref"]
        )
        old_event_path = cast(str, old_event_ref["path"])
        old_event = cast(
            dict[str, object],
            json.loads(
                (fixture.repo / Path(*old_event_path.split("/"))).read_text(encoding="utf-8")
            ),
        )
        roster_wire = cast(dict[str, object], old_event["reviewer_roster_ref"])
        roster_ref = ReviewerRosterRefV1(
            cast(str, roster_wire["path"]),
            cast(str, roster_wire["schema"]),
            bytes.fromhex(cast(str, roster_wire["raw_sha256"])),
        )
        role_bindings = tuple(
            ReviewerRoleBindingV1(
                cast(str, item["reviewer_id"]),
                tuple(cast(list[str], item["roles"])),
            )
            for item in cast(list[dict[str, object]], old_event["reviewer_role_bindings"])
        )
        source_bindings_v1 = tuple(
            SourceBindingDigestV1(
                cast(str, item["artifact_role"]),
                cast(str, item["path"]),
                cast(str | None, item["schema_or_null"]),
                bytes.fromhex(cast(str, item["raw_sha256"])),
            )
            for item in cast(list[dict[str, object]], old_event["source_binding_digests"])
        )
        review_evidence_v1 = tuple(
            AcceptanceEvidenceRefV1(
                cast(str, item["path"]),
                bytes.fromhex(cast(str, item["raw_sha256"])),
                (
                    cast(str, cast(dict[str, object], item["locator"])["kind"]),
                    cast(str, cast(dict[str, object], item["locator"])["value"])
                    if "value" in cast(dict[str, object], item["locator"])
                    else None,
                ),
            )
            for item in cast(list[dict[str, object]], old_event["review_evidence_refs"])
        )
        theorem_subject = AcceptanceSubjectPayloadV1(
            AcceptanceSubjectKind.CONTEXT_THEOREM_RECORD,
            [theorem_id.digest_bytes, [source_evidence.to_cbor()], rationale],
        )
        theorem_event_input = ReviewAcceptanceEventInputV1(
            AcceptanceSubjectKind.CONTEXT_THEOREM_RECORD,
            theorem_subject.identity().digest_bytes,
            roster_ref,
            role_bindings,
            ReviewMode(cast(str, old_event["review_mode"])),
            source_bindings_v1,
            review_evidence_v1,
        )
        theorem_event_leaf = ReviewAcceptanceEventLeafV1.from_input(theorem_event_input)
        theorem_event_raw = (
            json.dumps(theorem_event_leaf.to_wire(), separators=(",", ":")) + "\n"
        ).encode()
        theorem_event_path = (
            "sources/m2_5/authorities/review_acceptance_events/v1/"
            + theorem_event_leaf.event_id.as_text().removeprefix("ae.v1/")
            + ".json"
        )
        fixture.write_repo(theorem_event_path, theorem_event_raw)
        theorem_event_ref = ReviewEventRefV1(
            theorem_event_path,
            hashlib.sha256(theorem_event_raw).digest(),
            theorem_event_leaf.event_id.as_text(),
        )
        theorem_record_id = compute_authority_identity(
            AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
            [
                "manafold.m2.5.c.context-proof-record-input.v1",
                theorem_id.digest_bytes,
                [source_evidence.to_cbor()],
                theorem_event_ref.to_cbor(),
                rationale,
            ],
        )

        def identity_wire(identity: AuthorityIdentityV1) -> dict[str, object]:
            return {
                "envelope_id": "mtgml.digest-envelope.v1",
                "algorithm_id": "sha-256",
                "semantic_domain": identity.semantic_domain,
                "payload_codec_id": "mtgml.canonical-cbor.v1",
                "input_schema_id": identity.input_schema_id,
                "digest_hex": identity.digest_bytes.hex(),
            }

        new_theorem = dict(theorem)
        new_theorem["theorem_id"] = identity_wire(theorem_id)
        new_theorem["record_id"] = identity_wire(theorem_record_id)
        new_theorem["subject_shape"] = subject_shape
        new_theorem["acceptance"] = {
            "decision": "human_accepted",
            "review_event_ref": theorem_event_ref.to_wire(),
        }
        base_source_bindings = cast(list[dict[str, object]], base_document["source_bindings"])
        for source_binding in base_source_bindings:
            if source_binding.get("artifact_role") == "acceptance_event_leaf":
                source_binding["path"] = theorem_event_path
                source_binding["raw_sha256"] = theorem_event_ref.raw_sha256.hex()
        base_document["context_proofs"] = [new_theorem]
        base_raw = json_bytes(base_document)
        base_file.write_bytes(base_raw)
        base_binding = ContextAuthoritySourceBindingV2(
            "base_authority_v1",
            base_path,
            "manafold.m2.5.c.interaction-review-authority.v1",
            hashlib.sha256(base_raw).digest(),
        )
        case["base_binding"] = base_binding
        case["member"] = replace(
            cast(ContextApplicationV2Record, case["record"]).members[0],
            candidate_id=candidate_id,
            candidate_identity_digest_reference=DigestReferenceV1(
                cast(str, candidates[0]["candidate_identity"]["envelope_id"]),
                cast(str, candidates[0]["candidate_identity"]["algorithm_id"]),
                cast(str, candidates[0]["candidate_identity"]["semantic_domain"]),
                cast(str, candidates[0]["candidate_identity"]["payload_codec_id"]),
                cast(str, candidates[0]["candidate_identity"]["input_schema_id"]),
                bytes.fromhex(cast(str, candidates[0]["candidate_identity"]["digest_hex"])),
            ),
            source_instance_id=source_instance_id,
            candidate_universe_binding=[
                "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
                "manafold.m2.5.c.interaction-candidate-universe.v2",
                candidate_universe_digest,
            ],
            context_binding_v1=subject_shape_cbor,
        )
        case["source_resolver"] = AuthoritySourceResolver(fixture.repo, rev3_archive=archive)
        zero_ref = ReviewEventRefV3(
            "sources/m2_5/authorities/review_acceptance_events/v3/" + "00" * 32 + ".json",
            bytes(32),
            "ae.v3/" + "00" * 32,
        )
        application_id = ContextApplicationV2InputV1(
            theorem_record_id_bytes=theorem_record_id.digest_bytes,
            members=(case["member"],),
        ).identity()
        provisional = ContextApplicationV2Record.from_parts(
            application_id=application_id,
            theorem_record_id=theorem_record_id,
            members=(case["member"],),
            review_event_ref_v3=zero_ref,
        )
        _, application_record, _ = build_application_record_with_v3_event(case, provisional)
        case["record"] = application_record
        case["member"] = application_record.members[0]

        # Copy the pinned B2 snapshot and create host-binding source artifacts.
        b2_paths = (
            "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
            "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
            "sources/m2_5/closures/B2/classification_closure.v1.json",
        )
        for path in b2_paths:
            target = fixture.repo / Path(*path.split("/"))
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes((ROOT / Path(*path.split("/"))).read_bytes())
        model_path = cast(ContextAuthoritySourceBindingV2, case["base_binding"]).path.replace(
            "authorities/interaction_review_authority.v1.json",
            "closures/C/declared_interaction_model.v2.json",
        )
        model_raw = (fixture.repo / Path(*model_path.split("/"))).read_bytes()
        model_binding = HostBindingSourceBindingV2(
            "declared_model",
            model_path,
            "manafold.m2.5.c.declared-interaction-model.v2",
            hashlib.sha256(model_raw).digest(),
        )
        host_map_binding = HostBindingSourceBindingV2(
            "rev3_card_requirement_map", map_path, None, hashlib.sha256(map_raw).digest()
        )
        host_deck_binding = HostBindingSourceBindingV2(
            "rev3_deck_row_source_resolution", deck_path, None, hashlib.sha256(deck_raw).digest()
        )
        host_osi_binding = HostBindingSourceBindingV2(
            "rev3_osi_source_records", osi_path, None, hashlib.sha256(osi_raw).digest()
        )
        host_candidate_binding = HostBindingSourceBindingV2(
            "candidate_universe",
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            "manafold.m2.5.c.interaction-candidate-universe.v2",
            candidate_universe_digest,
        )
        host_pair_binding = HostBindingSourceBindingV2(
            "rev3_pair_aggregates", pair_path, None, hashlib.sha256(pair_raw).digest()
        )
        b2_catalog_binding = HostBindingSourceBindingV2(
            "b2_catalog",
            b2_paths[0],
            "manafold.m2.5.b2.requirement-family-catalog.v1",
            hashlib.sha256((fixture.repo / Path(*b2_paths[0].split("/"))).read_bytes()).digest(),
        )
        b2_classification_binding = HostBindingSourceBindingV2(
            "b2_classifications",
            b2_paths[1],
            "manafold.m2.5.b2.card-semantic-classifications.v1",
            hashlib.sha256((fixture.repo / Path(*b2_paths[1].split("/"))).read_bytes()).digest(),
        )
        b2_closure_binding = HostBindingSourceBindingV2(
            "b2_closure",
            b2_paths[2],
            "manafold.m2.5.b2.classification-closure.v1",
            hashlib.sha256((fixture.repo / Path(*b2_paths[2].split("/"))).read_bytes()).digest(),
        )
        roster_raw = json_bytes(
            {
                "schema": "manafold.m2.5.c.reviewer-roster.v1",
                "reviewers": [
                    {"reviewer_id": "alice", "roles": ["architecture_maintainer", "project_owner"]}
                ],
            }
        )
        roster_path = (
            "sources/m2_5/authorities/reviewer_rosters/v1/"
            + hashlib.sha256(roster_raw).hexdigest()
            + ".json"
        )
        fixture.write_repo(roster_path, roster_raw)
        roster_binding = HostBindingSourceBindingV2(
            "reviewer_roster_leaf",
            roster_path,
            "manafold.m2.5.c.reviewer-roster.v1",
            hashlib.sha256(roster_raw).digest(),
        )
        roster_ref = ReviewerRosterRefV1(
            roster_path, roster_binding.schema_or_null, roster_binding.raw_sha256
        )
        evidence_path = "docs/review/host-binding-full-e2e.md"
        evidence_raw = b"full linked host binding e2e\n"
        fixture.write_repo(evidence_path, evidence_raw)
        evidence_ref = AcceptanceEvidenceRefV1(
            evidence_path, hashlib.sha256(evidence_raw).digest(), ("whole_artifact", None)
        )
        host_event_sources = tuple(
            sorted(
                (
                    model_binding,
                    roster_binding,
                    host_candidate_binding,
                    host_pair_binding,
                    host_map_binding,
                    host_deck_binding,
                    host_osi_binding,
                    b2_catalog_binding,
                    b2_classification_binding,
                    b2_closure_binding,
                ),
                key=lambda item: encode_canonical(item.to_cbor()),
            )
        )
        claim_key = ApplicationMemberKeyV1(
            case["member"].candidate_id,
            case["member"].candidate_identity_digest_reference.digest_bytes,
            case["member"].source_instance_id,
        )
        claim_discoveries = []
        claim_realizations = []
        for position, (_family_id, host_id, row, participant_ref) in enumerate(
            ((family_a, "Token Triumph", 0, family_a), (family_b, "Grave Danger", 1, family_b))
        ):
            mapping_ref = HostBindingEvidenceRefV2(
                "rev3_card_requirement_map",
                map_path,
                None,
                host_map_binding.raw_sha256,
                ("csv_row", row),
            )
            deck_ref = HostBindingEvidenceRefV2(
                "rev3_deck_row_source_resolution",
                deck_path,
                None,
                host_deck_binding.raw_sha256,
                ("csv_row", row),
            )
            osi_ref = HostBindingEvidenceRefV2(
                "rev3_osi_source_records",
                osi_path,
                None,
                host_osi_binding.raw_sha256,
                ("jsonl_line", row),
            )
            b2_ref = HostBindingEvidenceRefV2(
                "b2_classifications",
                b2_paths[1],
                b2_classification_binding.schema_or_null,
                b2_classification_binding.raw_sha256,
                ("json_pointer", "/classifications/0"),
            )
            host = DiscoveryHostRefV1("rev3_deck", host_id)
            claim_discoveries.append(
                CrossDeckParticipantDiscoveryHostBindingV1(
                    claim_key,
                    position,
                    participant_ref,
                    "rev3_left_family" if position == 0 else "rev3_right_family",
                    host,
                    (mapping_ref,),
                )
            )
            claim_realizations.append(
                ParticipantHostRealizationV1(
                    claim_key,
                    position,
                    participant_ref,
                    host,
                    (HostRealizationWitnessV1(mapping_ref, deck_ref, osi_ref, (b2_ref,)),),
                )
            )
        claim = CrossDeckHostBindingClaimV1(
            claim_key, tuple(claim_discoveries), tuple(claim_realizations), "cross_host"
        )

        def accepted_record(
            review_mode: ReviewMode,
        ) -> tuple[CrossDeckHostBindingClaimRecordV1, HostBindingSourceBindingV2]:
            event_input = HostBindingAcceptanceEventInputV2(
                "cross_deck_host_binding_claim_record_v1",
                claim.identity().digest_bytes,
                roster_ref,
                (ReviewerRoleBindingV1("alice", ("architecture_maintainer", "project_owner")),),
                review_mode,
                "cross-deck-host-binding-review-checklist.v1",
                host_event_sources,
                (evidence_ref,),
            )
            leaf = HostBindingAcceptanceEventLeafV2.from_input(event_input)
            raw = (json.dumps(leaf.to_wire(), separators=(",", ":")) + "\n").encode()
            path = (
                "sources/m2_5/authorities/review_acceptance_events/v2/"
                + leaf.event_id.as_text().removeprefix("ae.v2/")
                + ".json"
            )
            fixture.write_repo(path, raw)
            event_ref = HostBindingAcceptanceEventRefV2(
                path, hashlib.sha256(raw).digest(), leaf.event_id.as_text()
            )
            return (
                CrossDeckHostBindingClaimRecordV1(claim, event_ref),
                HostBindingSourceBindingV2(
                    "acceptance_event_leaf_v2",
                    path,
                    "manafold.m2.5.c.review-acceptance-event.v2",
                    hashlib.sha256(raw).digest(),
                ),
            )

        first_record, first_event_binding = accepted_record(ReviewMode.SOLO_SEPARATE_SELF_REVIEW)
        second_record, second_event_binding = accepted_record(ReviewMode.MULTI_REVIEWER)
        b2_evidence = HostBindingEvidenceRefV2(
            "b2_classifications",
            b2_paths[1],
            b2_classification_binding.schema_or_null,
            b2_classification_binding.raw_sha256,
            ("json_pointer", "/classifications/0"),
        )
        sup_event_sources = tuple(
            binding
            for binding in host_event_sources
            if binding.artifact_role
            in {
                "declared_model",
                "reviewer_roster_leaf",
                "b2_catalog",
                "b2_classifications",
                "b2_closure",
            }
        )
        placeholder = HostBindingAcceptanceEventRefV2(
            "sources/m2_5/authorities/review_acceptance_events/v2/" + "00" * 32 + ".json",
            bytes(32),
            "ae.v2/" + "00" * 32,
        )
        provisional_sup = CrossDeckHostBindingClaimSupersessionV1(
            first_record.record_identity(),
            second_record.record_identity(),
            "source_revision",
            (b2_evidence,),
            placeholder,
        )
        sup_input = HostBindingAcceptanceEventInputV2(
            "cross_deck_host_binding_claim_supersession_v1",
            provisional_sup.identity().digest_bytes,
            roster_ref,
            (ReviewerRoleBindingV1("alice", ("architecture_maintainer", "project_owner")),),
            ReviewMode.SOLO_SEPARATE_SELF_REVIEW,
            "cross-deck-host-binding-review-checklist.v1",
            sup_event_sources,
            (evidence_ref,),
        )
        sup_leaf = HostBindingAcceptanceEventLeafV2.from_input(sup_input)
        sup_raw = (json.dumps(sup_leaf.to_wire(), separators=(",", ":")) + "\n").encode()
        sup_path = (
            "sources/m2_5/authorities/review_acceptance_events/v2/"
            + sup_leaf.event_id.as_text().removeprefix("ae.v2/")
            + ".json"
        )
        fixture.write_repo(sup_path, sup_raw)
        supersession = CrossDeckHostBindingClaimSupersessionV1(
            first_record.record_identity(),
            second_record.record_identity(),
            "source_revision",
            (b2_evidence,),
            HostBindingAcceptanceEventRefV2(
                sup_path, hashlib.sha256(sup_raw).digest(), sup_leaf.event_id.as_text()
            ),
        )
        sup_event_binding = HostBindingSourceBindingV2(
            "acceptance_event_leaf_v2",
            sup_path,
            "manafold.m2.5.c.review-acceptance-event.v2",
            hashlib.sha256(sup_raw).digest(),
        )
        base_host = HostBindingSourceBindingV2(
            "base_authority_v1", base_binding.path, base_binding.schema, base_binding.raw_sha256
        )
        host_document = {
            "schema": "manafold.m2.5.c.interaction-review-authority.v2",
            "base_authority_v1_binding": base_host.to_wire(),
            "source_bindings": [
                binding.to_wire()
                for binding in sorted(
                    (
                        base_host,
                        first_event_binding,
                        second_event_binding,
                        sup_event_binding,
                        *host_event_sources,
                    ),
                    key=lambda item: encode_canonical(item.to_cbor()),
                )
            ],
            "cross_deck_host_binding_claim_records": [
                first_record.to_wire(),
                second_record.to_wire(),
            ],
            "cross_deck_host_binding_claim_supersession_records": [supersession.to_wire()],
            "application_host_bindings": [],
        }
        host_path = "sources/m2_5/authorities/interaction_review_authority.v2.json"
        host_raw = (json.dumps(host_document, separators=(",", ":")) + "\n").encode()
        fixture.write_repo(host_path, host_raw)
        host_binding = ContextAuthoritySourceBindingV2(
            "host_binding_authority_v2",
            host_path,
            "manafold.m2.5.c.interaction-review-authority.v2",
            hashlib.sha256(host_raw).digest(),
        )
        claim_bindings = []
        for record in (first_record, second_record):
            path = (
                "sources/m2_5/authorities/cross_deck_host_binding_claims/v1/"
                + record.record_identity().as_text().removeprefix("hbcr.v1/")
                + ".json"
            )
            raw = (json.dumps(record.to_wire(), separators=(",", ":")) + "\n").encode()
            fixture.write_repo(path, raw)
            claim_bindings.append(
                ContextAuthoritySourceBindingV2(
                    "host_binding_claim_record",
                    path,
                    "manafold.m2.5.c.cross-deck-host-binding-claim-record.v1",
                    hashlib.sha256(raw).digest(),
                )
            )
        semantic_claim_path = (
            "sources/m2_5/authorities/cross_deck_host_binding_claims/v1/"
            + claim.identity().as_text().removeprefix("hbc.v1/")
            + ".json"
        )
        semantic_claim_wire = {
            "schema": "manafold.m2.5.c.cross-deck-host-binding-claim-record.v1",
            **second_record.to_wire(),
        }
        semantic_claim_raw = (
            json.dumps(semantic_claim_wire, separators=(",", ":")) + "\n"
        ).encode()
        fixture.write_repo(semantic_claim_path, semantic_claim_raw)
        semantic_claim_binding = ContextAuthoritySourceBindingV2(
            "host_binding_claim_record",
            semantic_claim_path,
            "manafold.m2.5.c.cross-deck-host-binding-claim-record.v1",
            hashlib.sha256(semantic_claim_raw).digest(),
        )
        return (
            case,
            source_resolver,
            application_record,
            claim,
            host_binding,
            (semantic_claim_binding,),
        )

    def _evaluator(
        self,
        source_resolver: AuthoritySourceResolver | object | None = None,
    ) -> ContextApplicationV2HostBindingEvaluator:
        if source_resolver is None:
            source_resolver = AuthoritySourceResolver(ROOT)
        return ContextApplicationV2HostBindingEvaluator(source_resolver)

    @staticmethod
    def _member_key_for_record(record: ContextApplicationV2Record) -> ApplicationMemberKeyV1:
        member = record.members[0]
        return ApplicationMemberKeyV1(
            member.candidate_id,
            member.candidate_identity_digest_reference.digest_bytes,
            member.source_instance_id,
        )

    def _read_model_for_case(
        self,
        case: dict[str, object],
        claim: CrossDeckHostBindingClaimV1,
        *,
        current: bool = True,
        status: HostBindingClaimRecordStatus = HostBindingClaimRecordStatus.CURRENT,
    ) -> HostBindingAuthorityV2ReadModel:
        claim_id = claim.identity().as_text()
        record_id = "hbcr.v1/" + "42" * 32
        base = cast(ContextAuthoritySourceBindingV2, case["base_binding"])
        member = claim.member_key
        candidate_values = cast(ContextApplicationV2Record, case["record"]).members[0]
        candidate = ContextAuthoritySourceBindingV2(
            "candidate_universe",
            cast(str, candidate_values.candidate_universe_binding[0]),
            cast(str, candidate_values.candidate_universe_binding[1]),
            cast(bytes, candidate_values.candidate_universe_binding[2]),
        )
        host_base = HostBindingSourceBindingV2(
            base.artifact_role,
            base.path,
            base.schema,
            base.raw_sha256,
        )
        host_candidate = HostBindingSourceBindingV2(
            candidate.artifact_role,
            candidate.path,
            candidate.schema,
            candidate.raw_sha256,
        )
        return HostBindingAuthorityV2ReadModel(
            base_authority_v1_binding=host_base,
            candidate_universe_binding=host_candidate,
            admitted_claims_by_id=((claim_id, claim),),
            current_claims_by_id=((claim_id, claim),) if current else (),
            current_claims_by_member=((member, claim_id),) if current else (),
            claim_record_status_by_record_id=((record_id, status),),
            claim_record_ids_by_claim_id=((claim_id, (record_id,)),),
            used_source_bindings=(),
        )

    def _container_with_link(
        self,
        case: dict[str, object],
        record: ContextApplicationV2Record,
        claim_id: str,
        *,
        host_binding: ContextAuthoritySourceBindingV2 | None = HOST_BINDING,
        supersessions: tuple[ContextApplicationV2SupersessionRecord, ...] = (),
    ) -> ContextApplicationAuthorityV2:
        member = record.members[0]
        candidate_binding = ContextAuthoritySourceBindingV2(
            "candidate_universe",
            cast(str, member.candidate_universe_binding[0]),
            cast(str, member.candidate_universe_binding[1]),
            cast(bytes, member.candidate_universe_binding[2]),
        )
        return ContextApplicationAuthorityV2(
            base_authority_v1_binding=cast(
                ContextAuthoritySourceBindingV2,
                case["base_binding"],
            ),
            host_binding_authority_v2_binding=host_binding,
            candidate_universe_binding=candidate_binding,
            source_bindings=(),
            context_application_v2_records=(record,),
            context_application_v2_supersession_records=supersessions,
            application_host_bindings_v2=(
                ApplicationHostBindingV2(
                    "context_application",
                    record.application_id,
                    (claim_id,),
                ),
            ),
        )

    def _evaluator_with_read_model(
        self,
        source_resolver: AuthoritySourceResolver,
        read_model: HostBindingAuthorityV2ReadModel,
    ) -> ContextApplicationV2HostBindingEvaluator:
        class AdmittedEvaluator(ContextApplicationV2HostBindingEvaluator):
            def _admit_host_binding(
                self,
                container: ContextApplicationAuthorityV2,
            ) -> HostBindingAuthorityV2ReadModel:
                del container
                return read_model

            def _validate_container_source_closure(
                self,
                container: ContextApplicationAuthorityV2,
            ) -> tuple[ContextAuthoritySourceBindingV2, ...]:
                del container
                return ()

        return AdmittedEvaluator(source_resolver)

    def test_non_context_authority_input_is_rejected_with_frozen_error(self) -> None:
        evaluator = self._evaluator()

        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            evaluator.evaluate(object())

        error = caught.exception
        self.assertEqual(error.code, "HOST_INTEGRATION_INPUT_INVALID")
        self.assertEqual(error.location, "container")
        self.assertIsNone(error.application_id)
        self.assertEqual(error.subject_ids, ())
        with self.assertRaises(FrozenInstanceError):
            error.code = "changed"

    def test_duplicate_application_host_binding_targets_are_rejected(self) -> None:
        links = tuple(
            sorted(
                (_link(7, 1), _link(7, 2)),
                key=lambda link: encode_canonical(link.to_cbor()),
            )
        )
        container = _empty_container(links=links)

        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            self._evaluator().evaluate(container)

        self.assertEqual(caught.exception.code, "APPLICATION_HOST_BINDING_DUPLICATE")
        self.assertEqual(caught.exception.application_id, _application_id(7))

    def test_unknown_application_host_binding_target_is_rejected(self) -> None:
        container = _empty_container(links=(_link(8, 1),))

        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            self._evaluator().evaluate(container)

        self.assertEqual(
            caught.exception.code,
            "APPLICATION_HOST_BINDING_UNKNOWN_APPLICATION",
        )
        self.assertEqual(caught.exception.application_id, _application_id(8))

    def test_caller_cannot_supply_trusted_currentness_or_claim_inputs(self) -> None:
        evaluator = self._evaluator()
        container = _empty_container()

        with self.assertRaises(TypeError):
            evaluator.evaluate(container, trusted_current=True)
        with self.assertRaises(TypeError):
            evaluator.evaluate(container, admitted_claims_by_id={})

    def test_structural_result_is_frozen_and_currentness_owned(self) -> None:
        class BoundaryEvaluator(ContextApplicationV2HostBindingEvaluator):
            def _validate_container_source_closure(
                self,
                container: ContextApplicationAuthorityV2,
            ) -> tuple[ContextAuthoritySourceBindingV2, ...]:
                del container
                return ()

        result = BoundaryEvaluator(AuthoritySourceResolver(ROOT)).evaluate(_empty_container())

        self.assertIsInstance(result, ContextApplicationV2HostBindingEvaluationResult)
        self.assertEqual(result.qualified_current_application_record_ids, ())
        self.assertEqual(result.application_host_binding_results, ())
        self.assertEqual(result.current_host_claim_ids, ())
        with self.assertRaises(FrozenInstanceError):
            result.current_host_claim_ids = ("hbc.v1/" + "00" * 32,)

    def test_member_key_uses_candidate_identity_and_source_instance(self) -> None:
        case = self._synthetic_case()
        record = cast(ContextApplicationV2Record, case["record"])
        evaluator = self._evaluator(case["source_resolver"])

        projections = evaluator._resolve_record_members(
            record,
            cast(ContextAuthoritySourceBindingV2, case["base_binding"]),
        )

        self.assertEqual(len(projections), 1)
        self.assertEqual(
            projections[0].member_key,
            ApplicationMemberKeyV1(
                candidate_id=record.members[0].candidate_id,
                candidate_identity_digest=(
                    record.members[0].candidate_identity_digest_reference.digest_bytes
                ),
                source_instance_id=record.members[0].source_instance_id,
            ),
        )
        self.assertTrue(projections[0].required)

    def test_applicability_uses_only_verified_scope_and_relation(self) -> None:
        case = self._synthetic_case()
        member = cast(ContextApplicationV2Record, case["record"]).members[0]
        evaluator = self._evaluator(case["source_resolver"])

        required = evaluator._project_verified_member(
            member,
            {"scope": "cross_deck", "relation": "directional_binary"},
        )
        non_required = evaluator._project_verified_member(
            member,
            {"scope": "intra_deck", "relation": "unordered_binary"},
        )

        self.assertTrue(required.required)
        self.assertFalse(non_required.required)

    def test_current_and_historical_links_use_the_same_required_subset(self) -> None:
        from context_application_v2_test_support import (
            build_application_with_v3_event,
            build_supersession_with_v3_event,
        )

        case = self._synthetic_case()
        source_resolver, record, _ = build_application_with_v3_event(self, case)
        claim = _cross_host_claim(self._member_key_for_record(record))
        semantic_input = ContextApplicationV2SupersessionInputV2(
            superseded_record_id_bytes=record.record_id.digest_bytes,
            replacement_record_id_bytes=None,
            replacement_record_kind=None,
            reason_code=SupersessionReason.AUTHORITY_REVOCATION,
            source_evidence_refs=(case["member"].member_evidence_refs[0],),
        )
        _, supersession, _ = build_supersession_with_v3_event(
            self,
            case,
            semantic_input,
        )
        link = ApplicationHostBindingV2(
            "context_application",
            record.application_id,
            (claim.identity().as_text(),),
        )
        candidate_binding = ContextAuthoritySourceBindingV2(
            "candidate_universe",
            cast(str, record.members[0].candidate_universe_binding[0]),
            cast(str, record.members[0].candidate_universe_binding[1]),
            cast(bytes, record.members[0].candidate_universe_binding[2]),
        )

        current_container = ContextApplicationAuthorityV2(
            base_authority_v1_binding=cast(
                ContextAuthoritySourceBindingV2,
                case["base_binding"],
            ),
            host_binding_authority_v2_binding=HOST_BINDING,
            candidate_universe_binding=candidate_binding,
            source_bindings=(),
            context_application_v2_records=(record,),
            context_application_v2_supersession_records=(),
            application_host_bindings_v2=(link,),
        )
        historical_container = replace(
            current_container,
            context_application_v2_supersession_records=(supersession,),
        )
        current = self._evaluator_with_read_model(
            source_resolver,
            self._read_model_for_case(case, claim),
        ).evaluate(current_container)
        historical = self._evaluator_with_read_model(
            source_resolver,
            self._read_model_for_case(
                case,
                claim,
                current=False,
                status=HostBindingClaimRecordStatus.REVOKED,
            ),
        ).evaluate(historical_container)

        self.assertEqual(
            current.qualified_current_application_record_ids,
            (record.record_id,),
        )
        self.assertEqual(
            current.application_host_binding_results[0].status.value,
            "qualified_current",
        )
        self.assertEqual(
            current.current_host_claim_ids,
            (claim.identity().as_text(),),
        )
        self.assertEqual(
            historical.application_host_binding_results[0].status.value, "historical_only"
        )
        self.assertEqual(historical.qualified_current_application_record_ids, ())
        self.assertEqual(historical.current_host_claim_ids, ())
        evaluator = self._evaluator(source_resolver)
        current_closure = evaluator._derive_application_closures(
            (record,),
            current.currentness,
            cast(ContextAuthoritySourceBindingV2, case["base_binding"]),
        )
        historical_closure = evaluator._derive_application_closures(
            (record,),
            historical.currentness,
            cast(ContextAuthoritySourceBindingV2, case["base_binding"]),
        )
        self.assertEqual(
            current_closure[0].required_member_keys,
            historical_closure[0].required_member_keys,
        )

    def test_real_mixed_candidate_records_keep_only_cross_deck_required(self) -> None:
        case, source_resolver, record, member_a, member_b, _candidate_binding = (
            self._mixed_application_case()
        )
        from context_application_v2_supersession import ContextApplicationV2CurrentnessResult

        base_binding = cast(ContextAuthoritySourceBindingV2, case["base_binding"])
        currentness = ContextApplicationV2CurrentnessResult(
            current_record_ids=(record.record_id,),
            superseded_record_ids=(),
            revoked_record_ids=(),
            successor_edges=(),
        )
        historical_currentness = ContextApplicationV2CurrentnessResult(
            current_record_ids=(),
            superseded_record_ids=(record.record_id,),
            revoked_record_ids=(),
            successor_edges=(),
        )
        evaluator = self._evaluator(source_resolver)
        current_closure = evaluator._derive_application_closures(
            (record,),
            currentness,
            base_binding,
        )[0]
        historical_closure = evaluator._derive_application_closures(
            (record,),
            historical_currentness,
            base_binding,
        )[0]
        required_key = ApplicationMemberKeyV1(
            member_a.candidate_id,
            member_a.candidate_identity_digest_reference.digest_bytes,
            member_a.source_instance_id,
        )
        non_required_key = ApplicationMemberKeyV1(
            member_b.candidate_id,
            member_b.candidate_identity_digest_reference.digest_bytes,
            member_b.source_instance_id,
        )
        self.assertEqual(current_closure.required_member_keys, (required_key,))
        self.assertEqual(historical_closure.required_member_keys, (required_key,))
        self.assertNotIn(non_required_key, current_closure.required_member_keys)
        self.assertNotIn(non_required_key, historical_closure.required_member_keys)

    def test_current_claim_admission_qualifies_current_application(self) -> None:
        case = self._synthetic_case()
        source_resolver, record, _ = build_application_with_v3_event(self, case)
        claim = _cross_host_claim(self._member_key_for_record(record))
        read_model = self._read_model_for_case(case, claim)
        container = self._container_with_link(case, record, claim.identity().as_text())

        result = self._evaluator_with_read_model(source_resolver, read_model).evaluate(container)

        self.assertEqual(result.qualified_current_application_record_ids, (record.record_id,))
        self.assertEqual(
            result.application_host_binding_results[0].status.value, "qualified_current"
        )
        self.assertEqual(result.current_host_claim_ids, (claim.identity().as_text(),))

    def test_current_composition_consumes_record_level_hbc_read_model(self) -> None:
        case = self._synthetic_case()
        source_resolver, record, _ = build_application_with_v3_event(self, case)
        claim = _cross_host_claim(self._member_key_for_record(record))
        read_model = self._read_model_for_case(case, claim)
        claim_id = claim.identity().as_text()
        old_record_id = "hbcr.v1/" + "31" * 32
        current_record_id = "hbcr.v1/" + "32" * 32
        read_model = replace(
            read_model,
            claim_record_status_by_record_id=(
                (old_record_id, HostBindingClaimRecordStatus.SUPERSEDED),
                (current_record_id, HostBindingClaimRecordStatus.CURRENT),
            ),
            claim_record_ids_by_claim_id=((claim_id, (old_record_id, current_record_id)),),
        )
        container = self._container_with_link(case, record, claim_id)

        result = self._evaluator_with_read_model(source_resolver, read_model).evaluate(container)

        self.assertEqual(result.qualified_current_application_record_ids, (record.record_id,))
        self.assertEqual(result.current_host_claim_ids, (claim_id,))
        self.assertEqual(tuple(dict(read_model.current_claims_by_id)), (claim_id,))

    def test_full_linked_real_admission_e2e_qualifies_current_application(self) -> None:
        case, source_resolver, record, claim, host_binding, claim_bindings = (
            self._full_linked_case()
        )
        base_binding = cast(ContextAuthoritySourceBindingV2, case["base_binding"])
        member = record.members[0]
        candidate_binding = ContextAuthoritySourceBindingV2(
            "candidate_universe",
            cast(str, member.candidate_universe_binding[0]),
            cast(str, member.candidate_universe_binding[1]),
            cast(bytes, member.candidate_universe_binding[2]),
        )
        link = ApplicationHostBindingV2(
            "context_application",
            record.application_id,
            (claim.identity().as_text(),),
        )
        provisional = ContextApplicationAuthorityV2(
            base_authority_v1_binding=base_binding,
            host_binding_authority_v2_binding=host_binding,
            candidate_universe_binding=candidate_binding,
            source_bindings=canonical_source_bindings(
                (base_binding, candidate_binding, host_binding, *claim_bindings)
            ),
            context_application_v2_records=(record,),
            context_application_v2_supersession_records=(),
            application_host_bindings_v2=(link,),
        )
        context_resolver = ContextApplicationV2Resolver(
            source_resolver,
            base_authority_binding=base_binding,
        )
        complete_closure = context_resolver.expected_container_source_closure_v2(provisional)
        container = replace(provisional, source_bindings=complete_closure)

        result = ContextApplicationV2HostBindingEvaluator(source_resolver).evaluate(container)

        self.assertEqual(result.qualified_current_application_record_ids, (record.record_id,))
        self.assertEqual(
            result.application_host_binding_results[0].status.value, "qualified_current"
        )
        self.assertEqual(result.current_host_claim_ids, (claim.identity().as_text(),))

    def test_canonicalized_container_permutations_have_identical_results(self) -> None:
        case, source_resolver, record, claim, host_binding, claim_bindings = (
            self._full_linked_case()
        )
        base_binding = cast(ContextAuthoritySourceBindingV2, case["base_binding"])
        member = record.members[0]
        candidate_binding = ContextAuthoritySourceBindingV2(
            "candidate_universe",
            cast(str, member.candidate_universe_binding[0]),
            cast(str, member.candidate_universe_binding[1]),
            cast(bytes, member.candidate_universe_binding[2]),
        )
        link = ApplicationHostBindingV2(
            "context_application",
            record.application_id,
            (claim.identity().as_text(),),
        )
        provisional = ContextApplicationAuthorityV2(
            base_authority_v1_binding=base_binding,
            host_binding_authority_v2_binding=host_binding,
            candidate_universe_binding=candidate_binding,
            source_bindings=canonical_source_bindings(
                (base_binding, candidate_binding, host_binding, *claim_bindings)
            ),
            context_application_v2_records=(record,),
            context_application_v2_supersession_records=(),
            application_host_bindings_v2=(link,),
        )
        context_resolver = ContextApplicationV2Resolver(
            source_resolver,
            base_authority_binding=base_binding,
        )
        complete = context_resolver.expected_container_source_closure_v2(provisional)
        canonical_container = replace(provisional, source_bindings=complete)
        rebuilt_from_permuted_upstream = replace(
            provisional,
            source_bindings=canonical_source_bindings(tuple(reversed(complete))),
        )
        evaluator = ContextApplicationV2HostBindingEvaluator(source_resolver)
        self.assertEqual(
            evaluator.evaluate(canonical_container),
            evaluator.evaluate(rebuilt_from_permuted_upstream),
        )

    def test_rejection_is_failure_atomic_and_fingerprint_stable(self) -> None:
        case = self._synthetic_case()
        source_resolver, record, _ = build_application_with_v3_event(self, case)
        claim = _cross_host_claim(self._member_key_for_record(record))
        read_model = self._read_model_for_case(
            case, claim, current=False, status=HostBindingClaimRecordStatus.SUPERSEDED
        )
        container = self._container_with_link(case, record, claim.identity().as_text())
        evaluator = self._evaluator_with_read_model(source_resolver, read_model)
        before = (
            container.to_wire(),
            tuple(item.to_cbor() for item in container.context_application_v2_records),
            tuple(item.to_cbor() for item in container.context_application_v2_supersession_records),
            _file_digests(case["fixture"].repo),
        )
        fingerprints = []
        for _ in range(2):
            with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
                evaluator.evaluate(container)
            error = caught.exception
            fingerprints.append(
                (
                    error.code,
                    error.location,
                    error.cause_code,
                    error.application_id,
                    error.record_id,
                    error.claim_id,
                    error.member_key,
                    error.subject_ids,
                )
            )
            self.assertEqual(
                (
                    container.to_wire(),
                    tuple(item.to_cbor() for item in container.context_application_v2_records),
                    tuple(
                        item.to_cbor()
                        for item in container.context_application_v2_supersession_records
                    ),
                    _file_digests(case["fixture"].repo),
                ),
                before,
            )
        self.assertEqual(fingerprints[0], fingerprints[1])

    def test_historical_noncurrent_claims_are_retained_without_current_qualification(self) -> None:
        case = self._synthetic_case()
        source_resolver, record, _ = build_application_with_v3_event(self, case)
        claim = _cross_host_claim(self._member_key_for_record(record))
        semantic_input = ContextApplicationV2SupersessionInputV2(
            superseded_record_id_bytes=record.record_id.digest_bytes,
            replacement_record_id_bytes=None,
            replacement_record_kind=None,
            reason_code=SupersessionReason.AUTHORITY_REVOCATION,
            source_evidence_refs=(record.members[0].member_evidence_refs[0],),
        )
        _, supersession, _ = build_supersession_with_v3_event(
            self,
            case,
            semantic_input,
        )
        container = self._container_with_link(
            case,
            record,
            claim.identity().as_text(),
            supersessions=(supersession,),
        )
        for status in (
            HostBindingClaimRecordStatus.SUPERSEDED,
            HostBindingClaimRecordStatus.REVOKED,
        ):
            with self.subTest(status=status.value):
                read_model = self._read_model_for_case(
                    case,
                    claim,
                    current=False,
                    status=status,
                )
                result = self._evaluator_with_read_model(
                    source_resolver,
                    read_model,
                ).evaluate(container)
                self.assertEqual(result.qualified_current_application_record_ids, ())
                self.assertEqual(
                    result.application_host_binding_results[0].status.value,
                    "historical_only",
                )
                self.assertEqual(result.current_host_claim_ids, ())

    def test_historical_unknown_claim_is_rejected(self) -> None:
        case = self._synthetic_case()
        source_resolver, record, _ = build_application_with_v3_event(self, case)
        claim = _cross_host_claim(self._member_key_for_record(record))
        semantic_input = ContextApplicationV2SupersessionInputV2(
            superseded_record_id_bytes=record.record_id.digest_bytes,
            replacement_record_id_bytes=None,
            replacement_record_kind=None,
            reason_code=SupersessionReason.AUTHORITY_REVOCATION,
            source_evidence_refs=(record.members[0].member_evidence_refs[0],),
        )
        _, supersession, _ = build_supersession_with_v3_event(
            self,
            case,
            semantic_input,
        )
        read_model = self._read_model_for_case(case, claim)
        read_model = replace(
            read_model,
            admitted_claims_by_id=(),
            current_claims_by_id=(),
            current_claims_by_member=(),
            claim_record_status_by_record_id=(),
            claim_record_ids_by_claim_id=(),
        )
        container = self._container_with_link(
            case,
            record,
            claim.identity().as_text(),
            supersessions=(supersession,),
        )

        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            self._evaluator_with_read_model(source_resolver, read_model).evaluate(container)
        self.assertEqual(caught.exception.code, "HOST_CLAIM_UNKNOWN")

    def test_current_stale_claim_is_rejected(self) -> None:
        case = self._synthetic_case()
        source_resolver, record, _ = build_application_with_v3_event(self, case)
        claim = _cross_host_claim(self._member_key_for_record(record))
        read_model = self._read_model_for_case(
            case,
            claim,
            current=False,
            status=HostBindingClaimRecordStatus.SUPERSEDED,
        )
        container = self._container_with_link(case, record, claim.identity().as_text())

        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            self._evaluator_with_read_model(source_resolver, read_model).evaluate(container)
        self.assertEqual(caught.exception.code, "HOST_CLAIM_NOT_CURRENT")

    def test_host_authority_binding_is_required_for_any_link(self) -> None:
        case = self._synthetic_case()
        source_resolver, record, _ = build_application_with_v3_event(self, case)
        claim = _cross_host_claim(self._member_key_for_record(record))
        container = self._container_with_link(
            case,
            record,
            claim.identity().as_text(),
            host_binding=None,
        )

        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            self._evaluator(source_resolver).evaluate(container)
        self.assertEqual(caught.exception.code, "HOST_AUTHORITY_BINDING_REQUIRED")

    def test_unused_host_authority_binding_is_rejected(self) -> None:
        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            self._evaluator().evaluate(_empty_container(host_binding=HOST_BINDING))
        self.assertEqual(caught.exception.code, "HOST_AUTHORITY_BINDING_UNEXPECTED")

    def test_host_snapshot_mismatch_is_rejected(self) -> None:
        case = self._synthetic_case()
        source_resolver, record, _ = build_application_with_v3_event(self, case)
        claim = _cross_host_claim(self._member_key_for_record(record))
        read_model = self._read_model_for_case(case, claim)
        read_model = replace(
            read_model,
            base_authority_v1_binding=HostBindingSourceBindingV2(
                read_model.base_authority_v1_binding.artifact_role,
                read_model.base_authority_v1_binding.path,
                read_model.base_authority_v1_binding.schema_or_null,
                bytes([9]) * 32,
            ),
        )
        container = self._container_with_link(case, record, claim.identity().as_text())

        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            self._evaluator_with_read_model(source_resolver, read_model).evaluate(container)
        self.assertEqual(caught.exception.code, "HOST_AUTHORITY_CROSS_SNAPSHOT_MISMATCH")

    def test_container_source_closure_is_reused_before_result(self) -> None:
        case = self._synthetic_case()
        base_binding = cast(ContextAuthoritySourceBindingV2, case["base_binding"])
        member = cast(ContextApplicationV2Record, case["record"]).members[0]
        candidate_binding = ContextAuthoritySourceBindingV2(
            "candidate_universe",
            cast(str, member.candidate_universe_binding[0]),
            cast(str, member.candidate_universe_binding[1]),
            cast(bytes, member.candidate_universe_binding[2]),
        )
        container = ContextApplicationAuthorityV2(
            base_authority_v1_binding=base_binding,
            host_binding_authority_v2_binding=None,
            candidate_universe_binding=candidate_binding,
            source_bindings=canonical_source_bindings((base_binding, candidate_binding)),
            context_application_v2_records=(),
            context_application_v2_supersession_records=(),
            application_host_bindings_v2=(),
        )

        result = self._evaluator(case["source_resolver"]).evaluate(container)

        self.assertEqual(result.qualified_current_application_record_ids, ())
        with self.assertRaises(ContextApplicationV2HostBindingError) as missing:
            self._evaluator(case["source_resolver"]).evaluate(
                replace(container, source_bindings=())
            )
        self.assertEqual(missing.exception.code, "HOST_SOURCE_CLOSURE_MISMATCH")
        with self.assertRaises(ContextApplicationV2HostBindingError) as extra:
            self._evaluator(case["source_resolver"]).evaluate(
                replace(
                    container,
                    source_bindings=canonical_source_bindings(
                        (base_binding, candidate_binding, HOST_BINDING)
                    ),
                )
            )
        self.assertEqual(extra.exception.code, "HOST_SOURCE_CLOSURE_MISMATCH")

    def test_container_closure_accepts_additive_host_authority_provenance(self) -> None:
        from test_authority_v2_validator import AuthorityV2DocumentTests

        case = self._synthetic_case()
        authority_case = AuthorityV2DocumentTests()
        authority_case.setUp()
        self.addCleanup(authority_case.tearDown)
        host_path = "sources/m2_5/authorities/interaction_review_authority.v2.json"
        host_raw = (json.dumps(authority_case.document, separators=(",", ":")) + "\n").encode()
        host_file = case["fixture"].repo / Path(*host_path.split("/"))
        host_file.parent.mkdir(parents=True, exist_ok=True)
        host_file.write_bytes(host_raw)
        host_binding = ContextAuthoritySourceBindingV2(
            "host_binding_authority_v2",
            host_path,
            "manafold.m2.5.c.interaction-review-authority.v2",
            hashlib.sha256(host_raw).digest(),
        )
        base_binding = cast(ContextAuthoritySourceBindingV2, case["base_binding"])
        member = cast(ContextApplicationV2Record, case["record"]).members[0]
        candidate_binding = ContextAuthoritySourceBindingV2(
            "candidate_universe",
            cast(str, member.candidate_universe_binding[0]),
            cast(str, member.candidate_universe_binding[1]),
            cast(bytes, member.candidate_universe_binding[2]),
        )
        container = ContextApplicationAuthorityV2(
            base_authority_v1_binding=base_binding,
            host_binding_authority_v2_binding=host_binding,
            candidate_universe_binding=candidate_binding,
            source_bindings=canonical_source_bindings(
                (base_binding, candidate_binding, host_binding)
            ),
            context_application_v2_records=(),
            context_application_v2_supersession_records=(),
            application_host_bindings_v2=(),
        )

        closure = self._evaluator(case["source_resolver"])._validate_container_source_closure(
            container
        )

        self.assertIn(host_binding, closure)

    def test_linked_container_requires_claim_record_provenance(self) -> None:
        from test_authority_v2_validator import AuthorityV2DocumentTests

        case = self._synthetic_case()
        authority_case = AuthorityV2DocumentTests()
        authority_case.setUp()
        self.addCleanup(authority_case.tearDown)
        host_path = "sources/m2_5/authorities/interaction_review_authority.v2.json"
        host_raw = (json.dumps(authority_case.document, separators=(",", ":")) + "\n").encode()
        host_file = case["fixture"].repo / Path(*host_path.split("/"))
        host_file.parent.mkdir(parents=True, exist_ok=True)
        host_file.write_bytes(host_raw)
        host_binding = ContextAuthoritySourceBindingV2(
            "host_binding_authority_v2",
            host_path,
            "manafold.m2.5.c.interaction-review-authority.v2",
            hashlib.sha256(host_raw).digest(),
        )
        base_binding = cast(ContextAuthoritySourceBindingV2, case["base_binding"])
        member = cast(ContextApplicationV2Record, case["record"]).members[0]
        candidate_binding = ContextAuthoritySourceBindingV2(
            "candidate_universe",
            cast(str, member.candidate_universe_binding[0]),
            cast(str, member.candidate_universe_binding[1]),
            cast(bytes, member.candidate_universe_binding[2]),
        )
        container = ContextApplicationAuthorityV2(
            base_authority_v1_binding=base_binding,
            host_binding_authority_v2_binding=host_binding,
            candidate_universe_binding=candidate_binding,
            source_bindings=canonical_source_bindings(
                (base_binding, candidate_binding, host_binding)
            ),
            context_application_v2_records=(),
            context_application_v2_supersession_records=(),
            application_host_bindings_v2=(_link(31, 24),),
        )

        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            self._evaluator(case["source_resolver"])._validate_container_source_closure(container)
        self.assertEqual(caught.exception.code, "HOST_SOURCE_CLOSURE_MISMATCH")

    def test_real_authority_v2_admission_path_returns_read_model(self) -> None:
        from test_authority_v2_validator import AuthorityV2DocumentTests

        authority_case = AuthorityV2DocumentTests()
        authority_case.setUp()
        self.addCleanup(authority_case.tearDown)
        host_path = "sources/m2_5/authorities/interaction_review_authority.v2.json"
        host_raw = (json.dumps(authority_case.document, separators=(",", ":")) + "\n").encode()
        host_file = authority_case.repo / Path(*host_path.split("/"))
        host_file.parent.mkdir(parents=True, exist_ok=True)
        host_file.write_bytes(host_raw)
        host_binding = ContextAuthoritySourceBindingV2(
            "host_binding_authority_v2",
            host_path,
            "manafold.m2.5.c.interaction-review-authority.v2",
            hashlib.sha256(host_raw).digest(),
        )
        container = _empty_container(host_binding=host_binding)
        evaluator = self._evaluator(AuthoritySourceResolver(authority_case.repo))

        read_model = evaluator._admit_host_binding(container)

        self.assertEqual(read_model.admitted_claims_by_id, ())
        self.assertEqual(read_model.current_claims_by_id, ())

    def test_nested_authority_source_errors_stay_inside_slice6_boundary(self) -> None:
        container = _empty_container(host_binding=HOST_BINDING)
        evaluator = self._evaluator()
        for nested_error in (
            HostBindingSourceError("synthetic host join failure"),
            ResolutionError(ResolutionStatus.FAIL, "NESTED_RESOLUTION_FAILURE", "synthetic"),
        ):
            with self.subTest(error=type(nested_error).__name__):
                with (
                    patch.object(
                        ContextApplicationV2Resolver,
                        "resolve_source_binding",
                        return_value=SimpleNamespace(json_value={}),
                    ),
                    patch.object(
                        AuthorityV2Validator,
                        "admit",
                        side_effect=nested_error,
                    ),
                    self.assertRaises(ContextApplicationV2HostBindingError) as caught,
                ):
                    evaluator._admit_host_binding(container)
                self.assertEqual(caught.exception.code, "HOST_AUTHORITY_INVALID")
                self.assertEqual(
                    caught.exception.cause_code,
                    (
                        "HOST_SOURCE_INVALID"
                        if isinstance(nested_error, HostBindingSourceError)
                        else "NESTED_RESOLUTION_FAILURE"
                    ),
                )

    def test_member_source_resolution_errors_are_wrapped_with_typed_boundary(self) -> None:
        case = self._synthetic_case()
        record = cast(ContextApplicationV2Record, case["record"])

        class FailingResolver:
            def resolve_candidate_source_instance(self, *args: object, **kwargs: object) -> object:
                del args, kwargs
                raise ResolutionError(
                    ResolutionStatus.FAIL,
                    "SYNTHETIC_MEMBER_SOURCE_FAILURE",
                    "synthetic source failure",
                )

        evaluator = self._evaluator(FailingResolver())
        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            evaluator._resolve_record_members(
                record,
                cast(ContextAuthoritySourceBindingV2, case["base_binding"]),
            )
        self.assertEqual(caught.exception.code, "HOST_INTEGRATION_INPUT_INVALID")
        self.assertEqual(caught.exception.cause_code, "SYNTHETIC_MEMBER_SOURCE_FAILURE")

    def test_exact_member_union_and_reviewed_host_relationship_are_validated(self) -> None:
        member_a = ApplicationMemberKeyV1("candidate-a", bytes([1]) * 32, "si/a")
        member_b = ApplicationMemberKeyV1("candidate-b", bytes([2]) * 32, "si/b")
        projected = (
            _ResolvedApplicationMember(member_a, "same_host", True),
            _ResolvedApplicationMember(member_b, "same_host", False),
        )
        claim_a = _claim(member_a, "Token Triumph", 1)
        claim_b = _claim(member_b, "Grave Danger", 2)
        claims = {
            claim_a.identity().as_text(): claim_a,
            claim_b.identity().as_text(): claim_b,
        }
        application = _application_id(22)
        mixed_link = ApplicationHostBindingV2(
            "context_application",
            application,
            tuple(sorted(claims)),
        )

        required_members = tuple(member for member in projected if member.required)
        _validate_link_member_union(
            ApplicationHostBindingV2(
                "context_application",
                application,
                (claim_a.identity().as_text(),),
            ),
            required_members,
            claims,
        )

        with self.assertRaises(ContextApplicationV2HostBindingError) as missing:
            _validate_link_member_union(
                mixed_link,
                required_members,
                claims,
            )
        self.assertEqual(missing.exception.code, "HOST_MEMBER_SET_MISMATCH")

        with self.assertRaises(ContextApplicationV2HostBindingError) as relationship:
            _validate_link_member_union(
                mixed_link,
                (
                    _ResolvedApplicationMember(member_a, "same_host", True),
                    _ResolvedApplicationMember(member_b, "cross_host", True),
                ),
                {
                    claim_a.identity().as_text(): claim_a,
                    claim_b.identity().as_text(): claim_b,
                },
            )
        self.assertEqual(relationship.exception.code, "HOST_RELATIONSHIP_MISMATCH")

    def test_noncanonical_link_order_is_rejected_at_constructor_boundary(self) -> None:
        first, second = _link(9, 1), _link(10, 2)
        ordered = tuple(sorted((first, second), key=lambda link: encode_canonical(link.to_cbor())))
        noncanonical = tuple(reversed(ordered))

        with self.assertRaises(AuthorityContractError):
            _empty_container(links=noncanonical)

    def test_forged_typed_noncanonical_container_fails_at_evaluator_boundary(self) -> None:
        valid = _empty_container()
        forged = object.__new__(ContextApplicationAuthorityV2)
        for field_name in (
            "base_authority_v1_binding",
            "host_binding_authority_v2_binding",
            "candidate_universe_binding",
            "source_bindings",
            "context_application_v2_records",
            "context_application_v2_supersession_records",
        ):
            object.__setattr__(forged, field_name, getattr(valid, field_name))
        object.__setattr__(forged, "application_host_bindings_v2", [_link(11, 1)])

        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            self._evaluator().evaluate(forged)

        self.assertEqual(caught.exception.code, "HOST_INTEGRATION_INPUT_INVALID")
        self.assertEqual(caught.exception.location, "application_host_bindings_v2")

    def test_currentness_failure_precedes_host_binding_source_access(self) -> None:
        from test_context_application_v2_validator import ContextApplicationV2IntegrationTests

        base = ContextApplicationV2IntegrationTests()
        base.setUp()
        self.addCleanup(base.doCleanups)
        self.addCleanup(base.tearDown)
        case = base._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
        )
        source_resolver, application_a, _ = build_application_with_v3_event(self, case)
        _, application_b, _ = build_application_variant_with_v3_event(
            self,
            case,
            "cycle-b",
        )

        def supersession(
            source: ContextApplicationV2Record,
            replacement: ContextApplicationV2Record,
        ) -> ContextApplicationV2SupersessionRecord:
            semantic_input = ContextApplicationV2SupersessionInputV2(
                superseded_record_id_bytes=source.record_id.digest_bytes,
                replacement_record_id_bytes=replacement.record_id.digest_bytes,
                replacement_record_kind="context_application_v2_record",
                reason_code=SupersessionReason.SOURCE_REVISION,
                source_evidence_refs=(case["member"].member_evidence_refs[0],),
            )
            _, record, _ = build_supersession_with_v3_event(
                self,
                case,
                semantic_input,
            )
            return record

        a_to_b = supersession(application_a, application_b)
        b_to_a = supersession(application_b, application_a)
        records = tuple(
            sorted(
                (application_a, application_b),
                key=lambda record: encode_canonical(record.to_cbor()),
            )
        )
        supersessions = tuple(
            sorted(
                (a_to_b, b_to_a),
                key=lambda record: encode_canonical(record.to_cbor()),
            )
        )
        container = ContextApplicationAuthorityV2(
            base_authority_v1_binding=cast(
                ContextAuthoritySourceBindingV2,
                case["base_binding"],
            ),
            host_binding_authority_v2_binding=HOST_BINDING,
            candidate_universe_binding=CANDIDATE_BINDING,
            source_bindings=(),
            context_application_v2_records=records,
            context_application_v2_supersession_records=supersessions,
            application_host_bindings_v2=(),
        )
        fixture = case["fixture"]
        before = _file_digests(fixture.repo)

        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            self._evaluator(source_resolver).evaluate(container)

        error = caught.exception
        self.assertEqual(error.code, "APPLICATION_CURRENTNESS_FAILED")
        self.assertEqual(error.cause_code, "SUPERSESSION_CYCLE")
        self.assertEqual(_file_digests(fixture.repo), before)


if __name__ == "__main__":
    unittest.main()
