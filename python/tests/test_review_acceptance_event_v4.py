from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

from mtgml.authority import (
    ACCEPTANCE_CHECKLIST_V3,
    ACCEPTANCE_EVENT_INPUT_SCHEMA_V4,
    ACCEPTANCE_EVENT_SCHEMA_V4,
    ACCEPTANCE_SUBJECT_INPUT_SCHEMA_V4,
    ACCEPTANCE_SUBJECT_SCHEMA_V4,
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKindV4,
    AcceptanceSubjectPayloadV4,
    AuthorityContractError,
    AuthorityIdentityKind,
    DigestReferenceV1,
    ReviewAcceptanceEventInputV4,
    ReviewAcceptanceEventLeafV4,
    ReviewAuthoritySourceBindingV4,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewEventRefV4,
    ReviewMode,
    SourceBindingDigestV1,
    V1DependencySourceBindingToV4,
    compute_authority_identity,
    validate_v4_event_source_bindings,
)
from mtgml.persistence import encode_canonical

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
from authority_source_resolver import AuthoritySourceResolver
from review_acceptance_v4 import (
    ReviewAcceptanceV4BindingError,
    bind_review_acceptance_v4,
)

ROOT = Path(__file__).resolve().parents[2]
ZERO = bytes(32)
REQUIRED_ROLES = (
    "architecture_maintainer",
    "conformance_maintainer",
    "information_safety_reviewer",
    "rules_authority_maintainer",
)


def roster_ref() -> ReviewerRosterRefV1:
    return ReviewerRosterRefV1(
        path=f"sources/m2_5/authorities/reviewer_rosters/v1/{ZERO.hex()}.json",
        schema="manafold.m2.5.c.reviewer-roster.v1",
        raw_sha256=ZERO,
    )


def evidence() -> AcceptanceEvidenceRefV1:
    return AcceptanceEvidenceRefV1(
        path="docs/review/authority.md",
        raw_sha256=ZERO,
        locator=("whole_artifact", None),
    )


def source_bindings() -> tuple[ReviewAuthoritySourceBindingV4, ...]:
    values = (
        ReviewAuthoritySourceBindingV4(
            "declared_model",
            "sources/m2_5/closures/C/declared_interaction_model.v2.json",
            "manafold.m2.5.c.declared-interaction-model.v2",
            ZERO,
        ),
        ReviewAuthoritySourceBindingV4(
            "reviewer_roster_leaf",
            f"sources/m2_5/authorities/reviewer_rosters/v1/{ZERO.hex()}.json",
            "manafold.m2.5.c.reviewer-roster.v1",
            ZERO,
        ),
    )
    return tuple(sorted(values, key=lambda value: encode_canonical(value.to_cbor())))


def subject_payload(kind: AcceptanceSubjectKindV4) -> AcceptanceSubjectPayloadV4:
    if kind is AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD:
        payload = [kind.value, ZERO, ZERO, "required_interaction", [["member"]]]
    elif kind is AcceptanceSubjectKindV4.CONTEXT_APPLICATION_V3_RECORD:
        payload = [kind.value, ZERO, ZERO, [["member"]]]
    else:
        record_kind = (
            "relation_application_v2_record"
            if kind is AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_SUPERSESSION_RECORD
            else "context_application_v3_record"
        )
        payload = [
            kind.value,
            ZERO,
            ZERO,
            None,
            record_kind,
            None,
            "authority_revocation",
            [["model", "docs/review/authority.md", ["whole_artifact", None], ZERO]],
        ]
    return AcceptanceSubjectPayloadV4(kind, payload)


def event_input(kind: AcceptanceSubjectKindV4) -> ReviewAcceptanceEventInputV4:
    subject = subject_payload(kind)
    return ReviewAcceptanceEventInputV4(
        subject_kind=kind,
        subject_payload_digest_reference=DigestReferenceV1.from_identity(subject.identity()),
        reviewer_roster_ref=roster_ref(),
        reviewer_role_bindings=(ReviewerRoleBindingV1("reviewer", REQUIRED_ROLES),),
        review_mode=ReviewMode.MULTI_REVIEWER,
        source_binding_digests=source_bindings(),
        review_evidence_refs=(evidence(),),
    )


class FakeV4Resolver:
    def __init__(self, event: ReviewAcceptanceEventLeafV4) -> None:
        self.event = event
        self.event_ref: ReviewEventRefV4 | None = None
        self.source_bindings: list[ReviewAuthoritySourceBindingV4] = []
        self.evidence: list[AcceptanceEvidenceRefV1] = []

    def resolve_acceptance_event_leaf_v4(
        self, reference: ReviewEventRefV4
    ) -> ReviewAcceptanceEventLeafV4:
        self.event_ref = reference
        return self.event

    def resolve_v4_source_binding(self, binding: ReviewAuthoritySourceBindingV4) -> object:
        self.source_bindings.append(binding)
        return object()

    def resolve_v4_acceptance_evidence(self, evidence: AcceptanceEvidenceRefV1) -> object:
        self.evidence.append(evidence)
        return object()


class ReviewAcceptanceEventV4Tests(unittest.TestCase):
    def test_persisted_wire_fixture_matches_recomputed_event(self) -> None:
        fixture = json.loads(
            (ROOT / "conformance/fixtures/authority/review_acceptance_event.v4.json").read_text(
                encoding="utf-8"
            )
        )
        event = ReviewAcceptanceEventLeafV4.from_input(
            event_input(AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD)
        )
        self.assertEqual(event.to_wire(), fixture)

    def test_generic_binding_checks_subject_and_exact_source_closure(self) -> None:
        subject = subject_payload(AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD)
        event = ReviewAcceptanceEventLeafV4.from_input(event_input(subject.subject_kind))
        event_ref = ReviewEventRefV4(
            f"sources/m2_5/authorities/review_acceptance_events/v4/{event.event_id.digest_bytes.hex()}.json",
            ZERO,
            event.event_id.as_text(),
        )
        resolver = FakeV4Resolver(event)
        result = bind_review_acceptance_v4(subject, event_ref, resolver, source_bindings())
        self.assertIs(result.subject, subject)
        self.assertEqual(result.event_id, event.event_id.as_text())
        self.assertEqual(result.exact_event_closure, source_bindings())
        self.assertEqual(resolver.event_ref, event_ref)
        self.assertEqual(len(resolver.source_bindings), len(source_bindings()))
        self.assertEqual(len(resolver.evidence), 1)
        with self.assertRaises(ReviewAcceptanceV4BindingError):
            bind_review_acceptance_v4(
                subject,
                event_ref,
                resolver,
                source_bindings()[:-1],
            )

    def test_generic_binding_resolves_event_roster_sources_and_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            model_path = root / "sources/m2_5/closures/C/declared_interaction_model.v2.json"
            evidence_path = root / "docs/review/authority.md"
            model_path.parent.mkdir(parents=True)
            evidence_path.parent.mkdir(parents=True)
            model_bytes = b'{"schema":"manafold.m2.5.c.declared-interaction-model.v2"}'
            roster_bytes = json.dumps(
                {
                    "schema": "manafold.m2.5.c.reviewer-roster.v1",
                    "reviewers": [{"reviewer_id": "reviewer", "roles": list(REQUIRED_ROLES)}],
                },
                separators=(",", ":"),
            ).encode()
            evidence_bytes = b"review evidence"
            model_path.write_bytes(model_bytes)
            evidence_path.write_bytes(evidence_bytes)
            model_digest = hashlib.sha256(model_bytes).digest()
            roster_digest = hashlib.sha256(roster_bytes).digest()
            evidence_digest = hashlib.sha256(evidence_bytes).digest()
            roster_path = root / (
                "sources/m2_5/authorities/reviewer_rosters/v1/" + roster_digest.hex() + ".json"
            )
            roster_path.parent.mkdir(parents=True)
            roster_path.write_bytes(roster_bytes)
            subject = subject_payload(AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD)
            sources = (
                ReviewAuthoritySourceBindingV4(
                    "declared_model",
                    "sources/m2_5/closures/C/declared_interaction_model.v2.json",
                    "manafold.m2.5.c.declared-interaction-model.v2",
                    model_digest,
                ),
                ReviewAuthoritySourceBindingV4(
                    "reviewer_roster_leaf",
                    roster_path.relative_to(root).as_posix(),
                    "manafold.m2.5.c.reviewer-roster.v1",
                    roster_digest,
                ),
            )
            event = ReviewAcceptanceEventLeafV4.from_input(
                ReviewAcceptanceEventInputV4(
                    subject_kind=subject.subject_kind,
                    subject_payload_digest_reference=DigestReferenceV1.from_identity(
                        subject.identity()
                    ),
                    reviewer_roster_ref=ReviewerRosterRefV1(
                        roster_path.relative_to(root).as_posix(),
                        "manafold.m2.5.c.reviewer-roster.v1",
                        roster_digest,
                    ),
                    reviewer_role_bindings=(ReviewerRoleBindingV1("reviewer", REQUIRED_ROLES),),
                    review_mode=ReviewMode.MULTI_REVIEWER,
                    source_binding_digests=sources,
                    review_evidence_refs=(
                        AcceptanceEvidenceRefV1(
                            evidence_path.relative_to(root).as_posix(),
                            evidence_digest,
                            ("whole_artifact", None),
                        ),
                    ),
                )
            )
            event_path = root / (
                "sources/m2_5/authorities/review_acceptance_events/v4/"
                + event.event_id.digest_bytes.hex()
                + ".json"
            )
            event_path.parent.mkdir(parents=True)
            event_bytes = json.dumps(event.to_wire(), separators=(",", ":")).encode()
            event_path.write_bytes(event_bytes)
            reference = ReviewEventRefV4(
                event_path.relative_to(root).as_posix(),
                hashlib.sha256(event_bytes).digest(),
                event.event_id.as_text(),
            )
            resolver = AuthoritySourceResolver(root)
            result = bind_review_acceptance_v4(subject, reference, resolver, sources)
            self.assertEqual(result.event.event_id, event.event_id)

    def test_all_four_subjects_use_v4_identity_contracts(self) -> None:
        for kind in AcceptanceSubjectKindV4:
            with self.subTest(kind=kind):
                subject = subject_payload(kind)
                self.assertEqual(subject.identity().prefix, "asp.v4/")
                self.assertEqual(subject.identity().semantic_domain, ACCEPTANCE_SUBJECT_SCHEMA_V4)
                self.assertEqual(
                    subject.identity().input_schema_id, ACCEPTANCE_SUBJECT_INPUT_SCHEMA_V4
                )
                event = ReviewAcceptanceEventLeafV4.from_input(event_input(kind))
                self.assertEqual(event.event_id.prefix, "ae.v4/")
                self.assertEqual(event.event_id.semantic_domain, ACCEPTANCE_EVENT_SCHEMA_V4)
                self.assertEqual(event.event_id.input_schema_id, ACCEPTANCE_EVENT_INPUT_SCHEMA_V4)
                self.assertEqual(event.to_wire()["checklist_id"], ACCEPTANCE_CHECKLIST_V3)

    def test_required_reviewer_roles_and_nonempty_evidence_are_enforced(self) -> None:
        valid = event_input(AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD)
        missing_role = ReviewerRoleBindingV1(
            "reviewer",
            ("architecture_maintainer", "conformance_maintainer", "rules_authority_maintainer"),
        )
        with self.assertRaises(AuthorityContractError):
            ReviewAcceptanceEventInputV4(
                **{**valid.__dict__, "reviewer_role_bindings": (missing_role,)}
            )
        with self.assertRaises(AuthorityContractError):
            ReviewAcceptanceEventInputV4(**{**valid.__dict__, "review_evidence_refs": ()})

    def test_v1_dependency_projection_maps_only_the_four_raw_rev3_paths(self) -> None:
        expected = {
            "derived/Pair_Interaction_Census_REV3.csv": "rev3_candidate_census",
            "inputs/deck_row_source_resolution_REV3.csv": "rev3_deck_row_source_resolution",
            "source/raw/oracle_cards_selected_REV3.jsonl": "rev3_osi_source_records",
            "source/raw/source_record_index_REV3.csv": "rev3_source_index",
        }
        for path, role in expected.items():
            projected = V1DependencySourceBindingToV4(
                SourceBindingDigestV1("rev3_source", path, None, ZERO)
            )
            self.assertEqual(projected.artifact_role, role)
            self.assertEqual(projected.path, path)
            self.assertIsNone(projected.schema)
            self.assertEqual(projected.raw_sha256, ZERO)
        with self.assertRaises(AuthorityContractError):
            ReviewAuthoritySourceBindingV4(
                "rev3_candidate_census", "inputs/deck_row_source_resolution_REV3.csv", None, ZERO
            )
        with self.assertRaises(AuthorityContractError):
            ReviewAuthoritySourceBindingV4(
                "rev3_candidate_census", "derived/Pair_Interaction_Census_REV3.csv", "wrong", ZERO
            )
        with self.assertRaises(AuthorityContractError):
            ReviewAuthoritySourceBindingV4(
                "rev3_deck_row_source_resolution",
                "derived/Pair_Interaction_Census_REV3.csv",
                None,
                ZERO,
            )
        with self.assertRaises(AuthorityContractError):
            V1DependencySourceBindingToV4(
                SourceBindingDigestV1(
                    "rev3_source", "inputs/interaction_model_v1.json", "interaction-model.v1", ZERO
                )
            )

    def test_review_event_reference_and_source_registry_fail_closed(self) -> None:
        event = ReviewAcceptanceEventLeafV4.from_input(
            event_input(AcceptanceSubjectKindV4.CONTEXT_APPLICATION_V3_RECORD)
        )
        reference = ReviewEventRefV4(
            path=f"sources/m2_5/authorities/review_acceptance_events/v4/{event.event_id.digest_bytes.hex()}.json",
            raw_sha256=ZERO,
            event_id=event.event_id.as_text(),
        )
        self.assertEqual(reference.to_cbor()[2][0], "event_id")
        with self.assertRaises(AuthorityContractError):
            ReviewEventRefV4(
                "sources/m2_5/authorities/review_acceptance_events/v3/" + "0" * 64 + ".json",
                ZERO,
                "ae.v3/" + "0" * 64,
            )
        with self.assertRaises(AuthorityContractError):
            ReviewAuthoritySourceBindingV4("unknown_role", "x", None, ZERO)
        own_path = (
            "sources/m2_5/authorities/review_acceptance_events/v4/"
            + event.event_id.digest_bytes.hex()
            + ".json"
        )
        self_binding = ReviewAuthoritySourceBindingV4(
            "acceptance_event_leaf_v4", own_path, ACCEPTANCE_EVENT_SCHEMA_V4, ZERO
        )
        with self.assertRaises(AuthorityContractError):
            validate_v4_event_source_bindings(event.event_id, (self_binding,))

    def test_negative_fixture_matrix_is_executable_and_fails_closed(self) -> None:
        fixture = json.loads(
            (
                ROOT
                / (
                    "conformance/fixtures/authority/"
                    "review_acceptance_event_v4_negative_matrix.v1.json"
                )
            ).read_text(encoding="utf-8")
        )
        base = event_input(AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD)
        for case in fixture["negative"]:
            mutation = case["mutation"]
            with self.subTest(case=case["name"]):
                if mutation == "self_binding_role":
                    event = ReviewAcceptanceEventLeafV4.from_input(base)
                    own_path = (
                        "sources/m2_5/authorities/review_acceptance_events/v4/"
                        + event.event_id.digest_bytes.hex()
                        + ".json"
                    )
                    self_binding = ReviewAuthoritySourceBindingV4(
                        "acceptance_event_leaf_v4", own_path, ACCEPTANCE_EVENT_SCHEMA_V4, ZERO
                    )
                    with self.assertRaises(AuthorityContractError):
                        validate_v4_event_source_bindings(event.event_id, (self_binding,))
                    continue
                if mutation == "wrong_event_reference_namespace":
                    with self.assertRaises(AuthorityContractError):
                        ReviewEventRefV4(
                            "sources/m2_5/authorities/review_acceptance_events/v3/"
                            + "0" * 64
                            + ".json",
                            ZERO,
                            "ae.v3/" + "0" * 64,
                        )
                    continue
                values = copy.deepcopy(base.semantic_input())
                if mutation == "unknown_subject_kind":
                    values[1] = "not_a_v4_subject"
                elif mutation == "wrong_checklist":
                    values[7] = "interaction-authority-review-checklist.v2"
                elif mutation == "missing_required_reviewer_role":
                    values[5] = [["reviewer", ["architecture_maintainer"]]]
                elif mutation == "empty_review_evidence":
                    values[9] = []
                elif mutation == "unknown_source_role":
                    values[8][0][0] = "unknown_role"
                else:
                    raise AssertionError(f"unknown negative mutation {mutation}")
                with self.assertRaises(AuthorityContractError):
                    compute_authority_identity(
                        AuthorityIdentityKind.REVIEW_ACCEPTANCE_EVENT_V4, values
                    )


if __name__ == "__main__":
    unittest.main()
