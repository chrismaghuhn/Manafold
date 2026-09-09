from __future__ import annotations

import sys
import unittest
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from mtgml.authority import (
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKindV4,
    AcceptanceSubjectPayloadV4,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    DigestReferenceV1,
    RelationApplicationV2,
    RelationApplicationV2Record,
    ReviewAcceptanceEventInputV4,
    ReviewAcceptanceEventLeafV4,
    ReviewAuthoritySourceBindingV4,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewEventRefV4,
    ReviewMode,
)
from relation_application_v2_review_admission import (
    RelationApplicationV2ReviewAdmissionError,
    admit_relation_application_v2_record,
)
from test_relation_application_v2_contract import ZERO, event_ref, member
from test_relation_application_v2_validator import FakeSourceResolver, theorem_record


def source_binding() -> ReviewAuthoritySourceBindingV4:
    return ReviewAuthoritySourceBindingV4(
        "declared_model",
        "sources/m2_5/closures/C/declared_interaction_model.v2.json",
        "manafold.m2.5.c.declared-interaction-model.v2",
        b"m" * 32,
    )


def review_evidence() -> AcceptanceEvidenceRefV1:
    return AcceptanceEvidenceRefV1(
        path="sources/review-evidence.json",
        raw_sha256=b"e" * 32,
        locator=("whole_artifact", None),
    )


class FakeCurrentness:
    def __init__(self, status: str = "current", theorem: dict[str, object] | None = None) -> None:
        self.status = status
        self.theorem = theorem

    def require_current_relation_theorem(
        self, _theorem_id: AuthorityIdentityV1
    ) -> dict[str, object]:
        if self.status == "current":
            if self.theorem is None:
                raise RelationApplicationV2ReviewAdmissionError(
                    "RELATION_APPLICATION_V2_CURRENTNESS_FAILED", "theorem_record_id"
                )
            return self.theorem
        code = {
            "no-current": "RELATION_APPLICATION_V2_CURRENTNESS_FAILED",
            "ambiguous": "RELATION_APPLICATION_V2_CURRENTNESS_AMBIGUOUS",
            "revoked": "SUPERSEDED_AUTHORITY_USED",
        }.get(self.status, "SUPERSEDED_AUTHORITY_USED")
        raise RelationApplicationV2ReviewAdmissionError(code, "theorem_record_id")


class FakeAdmissionResolver(FakeSourceResolver):
    def __init__(self, event: ReviewAcceptanceEventLeafV4) -> None:
        super().__init__()
        self.event = event

    def resolve_acceptance_event_leaf_v4(
        self, _reference: ReviewEventRefV4
    ) -> ReviewAcceptanceEventLeafV4:
        return self.event

    def resolve_v4_source_binding(self, _binding: ReviewAuthoritySourceBindingV4) -> object:
        return object()

    def resolve_v4_acceptance_evidence(self, _evidence: AcceptanceEvidenceRefV1) -> object:
        return object()

    def expected_relation_application_v2_source_closure(
        self,
        _record: RelationApplicationV2Record,
        _reviewer_roster_ref: object,
    ) -> tuple[ReviewAuthoritySourceBindingV4, ...]:
        return tuple(self.event.source_binding_digests)


def valid_record() -> tuple[
    RelationApplicationV2Record, tuple[ReviewAuthoritySourceBindingV4, ...]
]:
    theorem_id = AuthorityIdentityV1(AuthorityIdentityKind.RELATION_THEOREM_RECORD, ZERO)
    application = RelationApplicationV2(
        theorem_record_id_bytes=theorem_id.digest_bytes,
        terminal_disposition="required_interaction",
        members=(member(),),
    )
    placeholder = RelationApplicationV2Record.from_parts(
        application_id=application.identity(),
        theorem_record_id=theorem_id,
        terminal_disposition="required_interaction",
        members=application.members,
        review_event_ref_v4=event_ref(),
    )
    subject = AcceptanceSubjectPayloadV4(
        AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD,
        placeholder.acceptance_free_subject_payload(),
    )
    roster_ref = ReviewerRosterRefV1(
        path="sources/m2_5/authorities/reviewer_rosters/v1/" + "72" * 32 + ".json",
        schema="manafold.m2.5.c.reviewer-roster.v1",
        raw_sha256=bytes.fromhex("72" * 32),
    )
    reviewer_binding = ReviewerRoleBindingV1(
        reviewer_id="reviewer",
        roles=(
            "architecture_maintainer",
            "conformance_maintainer",
            "information_safety_reviewer",
            "rules_authority_maintainer",
        ),
    )
    event_input = ReviewAcceptanceEventInputV4(
        subject_kind=subject.subject_kind,
        subject_payload_digest_reference=DigestReferenceV1.from_identity(subject.identity()),
        reviewer_roster_ref=roster_ref,
        reviewer_role_bindings=(reviewer_binding,),
        review_mode=ReviewMode.MULTI_REVIEWER,
        source_binding_digests=(source_binding(),),
        review_evidence_refs=(review_evidence(),),
    )
    event = ReviewAcceptanceEventLeafV4.from_input(event_input)
    record = RelationApplicationV2Record.from_parts(
        application_id=application.identity(),
        theorem_record_id=theorem_id,
        terminal_disposition="required_interaction",
        members=application.members,
        review_event_ref_v4=ReviewEventRefV4(
            path="sources/m2_5/authorities/review_acceptance_events/v4/"
            + event.event_id.digest_bytes.hex()
            + ".json",
            raw_sha256=b"a" * 32,
            event_id=event.event_id.as_text(),
        ),
    )
    return record, (source_binding(),)


class RelationApplicationV2ReviewAdmissionTests(unittest.TestCase):
    def test_rpar_v2_requires_current_theorem_before_v4_binding(self) -> None:
        record, expected = valid_record()
        subject = AcceptanceSubjectPayloadV4(
            AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD,
            record.acceptance_free_subject_payload(),
        )
        event_input = ReviewAcceptanceEventInputV4(
            subject_kind=subject.subject_kind,
            subject_payload_digest_reference=DigestReferenceV1.from_identity(subject.identity()),
            reviewer_roster_ref=ReviewerRosterRefV1(
                path="sources/m2_5/authorities/reviewer_rosters/v1/" + "72" * 32 + ".json",
                schema="manafold.m2.5.c.reviewer-roster.v1",
                raw_sha256=bytes.fromhex("72" * 32),
            ),
            reviewer_role_bindings=(
                ReviewerRoleBindingV1(
                    "reviewer",
                    (
                        "architecture_maintainer",
                        "conformance_maintainer",
                        "information_safety_reviewer",
                        "rules_authority_maintainer",
                    ),
                ),
            ),
            review_mode=ReviewMode.MULTI_REVIEWER,
            source_binding_digests=expected,
            review_evidence_refs=(review_evidence(),),
        )
        resolver = FakeAdmissionResolver(ReviewAcceptanceEventLeafV4.from_input(event_input))
        result = admit_relation_application_v2_record(
            record,
            resolver,
            theorem_record=theorem_record(),
            currentness=FakeCurrentness(theorem=theorem_record()),
            expected_source_bindings=expected,
        )
        self.assertEqual(result.record_id, record.record_id.as_text())

        with self.assertRaises(RelationApplicationV2ReviewAdmissionError) as raised:
            admit_relation_application_v2_record(
                record,
                resolver,
                theorem_record=theorem_record(),
                currentness=FakeCurrentness("superseded", theorem_record()),
                expected_source_bindings=expected,
            )
        self.assertEqual(raised.exception.code, "SUPERSEDED_AUTHORITY_USED")

    def test_wrong_subject_digest_and_missing_closure_fail_closed(self) -> None:
        record, expected = valid_record()
        wrong_subject = AcceptanceSubjectPayloadV4(
            AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD,
            [
                "relation_application_v2_record",
                b"x" * 32,
                record.theorem_record_id.digest_bytes,
                "required_interaction",
                [member().to_cbor()],
            ],
        )
        event = ReviewAcceptanceEventLeafV4.from_input(
            ReviewAcceptanceEventInputV4(
                subject_kind=wrong_subject.subject_kind,
                subject_payload_digest_reference=DigestReferenceV1.from_identity(
                    wrong_subject.identity()
                ),
                reviewer_roster_ref=ReviewerRosterRefV1(
                    path="sources/m2_5/authorities/reviewer_rosters/v1/" + "72" * 32 + ".json",
                    schema="manafold.m2.5.c.reviewer-roster.v1",
                    raw_sha256=bytes.fromhex("72" * 32),
                ),
                reviewer_role_bindings=(
                    ReviewerRoleBindingV1(
                        "reviewer",
                        (
                            "architecture_maintainer",
                            "conformance_maintainer",
                            "information_safety_reviewer",
                            "rules_authority_maintainer",
                        ),
                    ),
                ),
                review_mode=ReviewMode.MULTI_REVIEWER,
                source_binding_digests=expected,
                review_evidence_refs=(review_evidence(),),
            )
        )
        resolver = FakeAdmissionResolver(event)
        with self.assertRaises(RelationApplicationV2ReviewAdmissionError) as raised:
            admit_relation_application_v2_record(
                record,
                resolver,
                theorem_record=theorem_record(),
                currentness=FakeCurrentness(theorem=theorem_record()),
                expected_source_bindings=(),
            )
        self.assertIn(
            raised.exception.code,
            {"RPA_V2_SUBJECT_DIGEST_MISMATCH", "RPA_V2_SOURCE_CLOSURE_MISMATCH"},
        )

    def test_theorem_currentness_states_fail_closed_with_stable_categories(self) -> None:
        record, expected = valid_record()
        subject = AcceptanceSubjectPayloadV4(
            AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD,
            record.acceptance_free_subject_payload(),
        )
        event_input = ReviewAcceptanceEventInputV4(
            subject_kind=subject.subject_kind,
            subject_payload_digest_reference=DigestReferenceV1.from_identity(subject.identity()),
            reviewer_roster_ref=ReviewerRosterRefV1(
                path="sources/m2_5/authorities/reviewer_rosters/v1/" + "72" * 32 + ".json",
                schema="manafold.m2.5.c.reviewer-roster.v1",
                raw_sha256=bytes.fromhex("72" * 32),
            ),
            reviewer_role_bindings=(
                ReviewerRoleBindingV1(
                    "reviewer",
                    (
                        "architecture_maintainer",
                        "conformance_maintainer",
                        "information_safety_reviewer",
                        "rules_authority_maintainer",
                    ),
                ),
            ),
            review_mode=ReviewMode.MULTI_REVIEWER,
            source_binding_digests=expected,
            review_evidence_refs=(review_evidence(),),
        )
        resolver = FakeAdmissionResolver(ReviewAcceptanceEventLeafV4.from_input(event_input))
        for status, expected_code in (
            ("superseded", "SUPERSEDED_AUTHORITY_USED"),
            ("revoked", "SUPERSEDED_AUTHORITY_USED"),
            ("no-current", "RELATION_APPLICATION_V2_CURRENTNESS_FAILED"),
            ("ambiguous", "RELATION_APPLICATION_V2_CURRENTNESS_AMBIGUOUS"),
        ):
            with self.subTest(status=status):
                with self.assertRaises(RelationApplicationV2ReviewAdmissionError) as raised:
                    admit_relation_application_v2_record(
                        record,
                        resolver,
                        theorem_record=theorem_record(),
                        currentness=FakeCurrentness(status, theorem_record()),
                        expected_source_bindings=expected,
                    )
                self.assertEqual(raised.exception.code, expected_code)

    def test_v4_missing_reviewer_role_and_evidence_are_rejected(self) -> None:
        record, expected = valid_record()
        subject = AcceptanceSubjectPayloadV4(
            AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD,
            record.acceptance_free_subject_payload(),
        )
        roster_ref = ReviewerRosterRefV1(
            path="sources/m2_5/authorities/reviewer_rosters/v1/" + "72" * 32 + ".json",
            schema="manafold.m2.5.c.reviewer-roster.v1",
            raw_sha256=bytes.fromhex("72" * 32),
        )
        event_input = ReviewAcceptanceEventInputV4(
            subject_kind=subject.subject_kind,
            subject_payload_digest_reference=DigestReferenceV1.from_identity(subject.identity()),
            reviewer_roster_ref=roster_ref,
            reviewer_role_bindings=(
                ReviewerRoleBindingV1(
                    "reviewer",
                    (
                        "architecture_maintainer",
                        "conformance_maintainer",
                        "information_safety_reviewer",
                        "rules_authority_maintainer",
                    ),
                ),
            ),
            review_mode=ReviewMode.MULTI_REVIEWER,
            source_binding_digests=expected,
            review_evidence_refs=(review_evidence(),),
        )
        event = ReviewAcceptanceEventLeafV4.from_input(event_input)
        resolver = FakeAdmissionResolver(
            SimpleNamespace(
                event_id=event.event_id,
                subject_kind=event.subject_kind,
                subject_payload_digest_reference=event.subject_payload_digest_reference,
                reviewer_roster_ref=roster_ref,
                reviewer_role_bindings=(
                    ReviewerRoleBindingV1("reviewer", ("architecture_maintainer",)),
                ),
                source_binding_digests=event.source_binding_digests,
                review_evidence_refs=event.review_evidence_refs,
            )
        )
        with self.assertRaises(RelationApplicationV2ReviewAdmissionError) as raised:
            admit_relation_application_v2_record(
                record,
                resolver,
                theorem_record=theorem_record(),
                currentness=FakeCurrentness(theorem=theorem_record()),
                expected_source_bindings=expected,
            )
        self.assertEqual(raised.exception.code, "REVIEWER_ROLE_MISSING")

        resolver.event = SimpleNamespace(
            event_id=event.event_id,
            subject_kind=event.subject_kind,
            subject_payload_digest_reference=event.subject_payload_digest_reference,
            reviewer_roster_ref=roster_ref,
            reviewer_role_bindings=event.reviewer_role_bindings,
            source_binding_digests=event.source_binding_digests,
            review_evidence_refs=(),
        )
        with self.assertRaises(RelationApplicationV2ReviewAdmissionError) as raised:
            admit_relation_application_v2_record(
                record,
                resolver,
                theorem_record=theorem_record(),
                currentness=FakeCurrentness(theorem=theorem_record()),
                expected_source_bindings=expected,
            )
        self.assertEqual(raised.exception.code, "REVIEW_EVIDENCE_MISSING")

    def test_semantic_theorem_mapping_must_be_the_current_theorem_mapping(self) -> None:
        record, expected = valid_record()
        subject = AcceptanceSubjectPayloadV4(
            AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD,
            record.acceptance_free_subject_payload(),
        )
        event_input = ReviewAcceptanceEventInputV4(
            subject_kind=subject.subject_kind,
            subject_payload_digest_reference=DigestReferenceV1.from_identity(subject.identity()),
            reviewer_roster_ref=ReviewerRosterRefV1(
                path="sources/m2_5/authorities/reviewer_rosters/v1/" + "72" * 32 + ".json",
                schema="manafold.m2.5.c.reviewer-roster.v1",
                raw_sha256=bytes.fromhex("72" * 32),
            ),
            reviewer_role_bindings=(
                ReviewerRoleBindingV1(
                    "reviewer",
                    (
                        "architecture_maintainer",
                        "conformance_maintainer",
                        "information_safety_reviewer",
                        "rules_authority_maintainer",
                    ),
                ),
            ),
            review_mode=ReviewMode.MULTI_REVIEWER,
            source_binding_digests=expected,
            review_evidence_refs=(review_evidence(),),
        )
        resolver = FakeAdmissionResolver(ReviewAcceptanceEventLeafV4.from_input(event_input))
        with self.assertRaises(RelationApplicationV2ReviewAdmissionError) as raised:
            admit_relation_application_v2_record(
                record,
                resolver,
                theorem_record=theorem_record(roles=("affected", "source")),
                currentness=FakeCurrentness(theorem=theorem_record()),
                expected_source_bindings=expected,
            )
        self.assertEqual(raised.exception.code, "RELATION_APPLICATION_V2_THEOREM_MISMATCH")


if __name__ == "__main__":
    unittest.main()
