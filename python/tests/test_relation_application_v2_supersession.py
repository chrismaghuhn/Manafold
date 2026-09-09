from __future__ import annotations

import sys
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from mtgml.authority import (
    AcceptanceSubjectKindV4,
    AcceptanceSubjectPayloadV4,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    DigestReferenceV1,
    RelationApplicationV2Record,
    RelationApplicationV2SupersessionInputV2,
    RelationApplicationV2SupersessionRecord,
    ReviewAcceptanceEventInputV4,
    ReviewAcceptanceEventLeafV4,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewEventRefV4,
    ReviewMode,
    SupersessionReason,
)
from relation_application_v2_review_admission import RelationApplicationV2ReviewAdmissionError
from relation_application_v2_supersession import (
    RelationApplicationV2CurrentnessEvaluator,
    RelationApplicationV2SupersessionEdge,
    RelationApplicationV2SupersessionError,
    admit_relation_application_v2_supersession_record,
)
from test_relation_application_v2_contract import evidence
from test_relation_application_v2_review_admission import (
    FakeAdmissionResolver,
    FakeCurrentness,
    review_evidence,
    source_binding,
    theorem_record,
    valid_record,
)


class RelationApplicationV2SupersessionTests(unittest.TestCase):
    def test_supersession_identity_and_reason_rules_are_closed(self) -> None:
        supersession = RelationApplicationV2SupersessionInputV2(
            superseded_record_id_bytes=b"a" * 32,
            replacement_record_id_bytes=None,
            replacement_record_kind=None,
            reason_code=SupersessionReason.AUTHORITY_REVOCATION,
            source_evidence_refs=(evidence(),),
        )
        self.assertEqual(
            supersession.identity().kind, AuthorityIdentityKind.RELATION_SUPERSESSION_V2
        )
        with self.assertRaises(ValueError):
            RelationApplicationV2SupersessionInputV2(
                b"a" * 32,
                b"b" * 32,
                None,
                SupersessionReason.SOURCE_REVISION,
                (evidence(),),
            )

    def test_graph_rejects_self_edges_and_cycles(self) -> None:
        a = AuthorityIdentityV1(AuthorityIdentityKind.RELATION_APPLICATION_RECORD_V2, b"a" * 32)
        b = AuthorityIdentityV1(AuthorityIdentityKind.RELATION_APPLICATION_RECORD_V2, b"b" * 32)
        s1 = AuthorityIdentityV1(AuthorityIdentityKind.RELATION_SUPERSESSION_V2, b"1" * 32)
        s2 = AuthorityIdentityV1(AuthorityIdentityKind.RELATION_SUPERSESSION_V2, b"2" * 32)
        self_edge = RelationApplicationV2SupersessionEdge(
            s1, (s1,), a, a, SupersessionReason.SOURCE_REVISION
        )
        with self.assertRaises(RelationApplicationV2SupersessionError) as raised:
            RelationApplicationV2CurrentnessEvaluator._validate_edges(
                (self_edge,),
                {a.as_text(): object()},  # type: ignore[dict-item]
            )
        self.assertEqual(raised.exception.code, "SELF_SUPERSESSION")
        edge_a = RelationApplicationV2SupersessionEdge(
            s1, (s1,), a, b, SupersessionReason.SOURCE_REVISION
        )
        edge_b = RelationApplicationV2SupersessionEdge(
            s2, (s2,), b, a, SupersessionReason.SOURCE_REVISION
        )
        with self.assertRaises(RelationApplicationV2SupersessionError) as raised:
            RelationApplicationV2CurrentnessEvaluator._validate_cycles(
                RelationApplicationV2CurrentnessEvaluator._validate_edges(
                    (edge_a, edge_b),
                    {a.as_text(): object(), b.as_text(): object()},  # type: ignore[dict-item]
                )[1]
            )
        self.assertEqual(raised.exception.code, "SUPERSESSION_CYCLE")

    def test_supersession_source_is_superseded_and_replacement_remains_current_candidate(
        self,
    ) -> None:
        a = AuthorityIdentityV1(AuthorityIdentityKind.RELATION_APPLICATION_RECORD_V2, b"a" * 32)
        b = AuthorityIdentityV1(AuthorityIdentityKind.RELATION_APPLICATION_RECORD_V2, b"b" * 32)
        edge = RelationApplicationV2SupersessionEdge(
            AuthorityIdentityV1(AuthorityIdentityKind.RELATION_SUPERSESSION_V2, b"1" * 32),
            (),
            a,
            b,
            SupersessionReason.SOURCE_REVISION,
        )
        superseded_sources, _ = RelationApplicationV2CurrentnessEvaluator._validate_edges(
            (edge,),
            {a.as_text(): object(), b.as_text(): object()},  # type: ignore[dict-item]
        )
        self.assertEqual(superseded_sources, {a.as_text()})

    def test_two_current_records_in_one_application_group_are_ambiguous(self) -> None:
        record, _ = valid_record()
        subject = AcceptanceSubjectPayloadV4(
            AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD,
            record.acceptance_free_subject_payload(),
        )
        roster = ReviewerRosterRefV1(
            "sources/m2_5/authorities/reviewer_rosters/v1/" + "72" * 32 + ".json",
            "manafold.m2.5.c.reviewer-roster.v1",
            bytes.fromhex("72" * 32),
        )
        event = ReviewAcceptanceEventLeafV4.from_input(
            ReviewAcceptanceEventInputV4(
                subject.subject_kind,
                DigestReferenceV1.from_identity(subject.identity()),
                roster,
                (
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
                ReviewMode.MULTI_REVIEWER,
                (source_binding(),),
                (review_evidence(),),
            )
        )
        resolver = FakeAdmissionResolver(event)
        second = RelationApplicationV2Record.from_parts(
            record.application_id,
            record.theorem_record_id,
            record.terminal_disposition,
            record.members,
            ReviewEventRefV4(
                "sources/m2_5/authorities/review_acceptance_events/v4/" + "b" * 64 + ".json",
                b"b" * 32,
                "ae.v4/" + "b" * 64,
            ),
        )
        evaluator = RelationApplicationV2CurrentnessEvaluator(
            resolver,
            currentness=FakeCurrentness(theorem=theorem_record()),
        )
        with self.assertRaises(RelationApplicationV2SupersessionError) as raised:
            evaluator.evaluate((record, second), ())
        self.assertEqual(raised.exception.code, "CURRENTNESS_AMBIGUOUS")

    def test_stale_theorem_never_enters_live_current_records(self) -> None:
        record, _ = valid_record()
        with patch(
            "relation_application_v2_supersession.admit_relation_application_v2_record",
            side_effect=[
                RelationApplicationV2ReviewAdmissionError(
                    "SUPERSEDED_AUTHORITY_USED", "theorem_record_id"
                ),
                None,
            ],
        ):
            result = RelationApplicationV2CurrentnessEvaluator(
                object(), currentness=object()
            ).evaluate((record,), ())
        self.assertEqual(result.current_record_ids, ())

    def test_stale_theorem_does_not_mask_other_record_invalidity(self) -> None:
        record, _ = valid_record()
        with patch(
            "relation_application_v2_supersession.admit_relation_application_v2_record",
            side_effect=[
                RelationApplicationV2ReviewAdmissionError(
                    "SUPERSEDED_AUTHORITY_USED", "theorem_record_id"
                ),
                RelationApplicationV2ReviewAdmissionError(
                    "RELATION_APPLICATION_V2_IDENTITY_MISMATCH", "application_id"
                ),
            ],
        ), self.assertRaises(RelationApplicationV2SupersessionError) as raised:
            RelationApplicationV2CurrentnessEvaluator(object(), currentness=object()).evaluate(
                (record,), ()
            )
        self.assertEqual(raised.exception.code, "APPLICATION_REVIEW_ADMISSION_FAILED")

    def test_supersession_record_uses_its_own_v4_acceptance_before_graph_use(self) -> None:
        application_record, _ = valid_record()
        supersession_input = RelationApplicationV2SupersessionInputV2(
            application_record.record_id.digest_bytes,
            None,
            None,
            SupersessionReason.AUTHORITY_REVOCATION,
            (evidence(),),
        )
        placeholder = RelationApplicationV2SupersessionRecord.from_parts(
            supersession_input.identity(),
            application_record.record_id,
            None,
            SupersessionReason.AUTHORITY_REVOCATION,
            (evidence(),),
            ReviewEventRefV4(
                "sources/m2_5/authorities/review_acceptance_events/v4/" + "a" * 64 + ".json",
                b"a" * 32,
                "ae.v4/" + "a" * 64,
            ),
        )
        subject = AcceptanceSubjectPayloadV4(
            AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_SUPERSESSION_RECORD,
            placeholder.acceptance_free_subject_payload(),
        )
        roster = ReviewerRosterRefV1(
            "sources/m2_5/authorities/reviewer_rosters/v1/" + "72" * 32 + ".json",
            "manafold.m2.5.c.reviewer-roster.v1",
            bytes.fromhex("72" * 32),
        )
        event = ReviewAcceptanceEventLeafV4.from_input(
            ReviewAcceptanceEventInputV4(
                subject.subject_kind,
                DigestReferenceV1.from_identity(subject.identity()),
                roster,
                (
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
                ReviewMode.MULTI_REVIEWER,
                (source_binding(),),
                (review_evidence(),),
            )
        )
        record = RelationApplicationV2SupersessionRecord.from_parts(
            supersession_input.identity(),
            application_record.record_id,
            None,
            SupersessionReason.AUTHORITY_REVOCATION,
            (evidence(),),
            ReviewEventRefV4(
                "sources/m2_5/authorities/review_acceptance_events/v4/"
                + event.event_id.digest_bytes.hex()
                + ".json",
                b"b" * 32,
                event.event_id.as_text(),
            ),
        )

        class FakeSupersessionResolver(FakeAdmissionResolver):
            def expected_relation_application_v2_supersession_source_closure(
                self,
                _record: RelationApplicationV2SupersessionRecord,
                _reviewer_roster_ref: ReviewerRosterRefV1,
                _superseded: RelationApplicationV2Record,
                _replacement: RelationApplicationV2Record | None,
            ) -> tuple[object, ...]:
                return tuple(self.event.source_binding_digests)

        resolver = FakeSupersessionResolver(event)
        result = admit_relation_application_v2_supersession_record(
            record,
            resolver,
            endpoints={application_record.record_id.as_text(): application_record},
        )
        self.assertEqual(result.record_id, record.record_id)


if __name__ == "__main__":
    unittest.main()
