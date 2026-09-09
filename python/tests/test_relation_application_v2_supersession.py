from __future__ import annotations

import sys
import unittest
from pathlib import Path
from unittest.mock import patch

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
from test_relation_application_v2_contract import event_ref, evidence, member
from test_relation_application_v2_review_admission import (
    FakeAdmissionResolver,
    FakeCurrentness,
    review_evidence,
    source_binding,
    theorem_record,
    valid_record,
)


class MultiEventAdmissionResolver(FakeAdmissionResolver):
    def __init__(self, events: tuple[ReviewAcceptanceEventLeafV4, ...]) -> None:
        super().__init__(events[0])
        self._events = {event.event_id.as_text(): event for event in events}

    def _event_for(self, reference: ReviewEventRefV4) -> ReviewAcceptanceEventLeafV4:
        return self._events[reference.event_id]

    def resolve_acceptance_event_leaf_v4(
        self, reference: ReviewEventRefV4
    ) -> ReviewAcceptanceEventLeafV4:
        return self._event_for(reference)

    def expected_relation_application_v2_source_closure(
        self,
        record: RelationApplicationV2Record,
        _reviewer_roster_ref: object,
    ) -> tuple[object, ...]:
        return tuple(self._event_for(record.review_event_ref_v4).source_binding_digests)

    def expected_relation_application_v2_supersession_source_closure(
        self,
        record: RelationApplicationV2SupersessionRecord,
        _reviewer_roster_ref: object,
        _superseded: RelationApplicationV2Record,
        _replacement: RelationApplicationV2Record | None,
    ) -> tuple[object, ...]:
        return tuple(self._event_for(record.review_event_ref_v4).source_binding_digests)


def _review_event_ref(event: ReviewAcceptanceEventLeafV4, marker: int) -> ReviewEventRefV4:
    return ReviewEventRefV4(
        path=(
            "sources/m2_5/authorities/review_acceptance_events/v4/"
            + event.event_id.digest_bytes.hex()
            + ".json"
        ),
        raw_sha256=bytes([marker]) * 32,
        event_id=event.event_id.as_text(),
    )


def _reviewer_roster() -> ReviewerRosterRefV1:
    return ReviewerRosterRefV1(
        "sources/m2_5/authorities/reviewer_rosters/v1/" + "72" * 32 + ".json",
        "manafold.m2.5.c.reviewer-roster.v1",
        bytes.fromhex("72" * 32),
    )


def _event_for_subject(
    subject: AcceptanceSubjectPayloadV4, marker: int
) -> ReviewAcceptanceEventLeafV4:
    return ReviewAcceptanceEventLeafV4.from_input(
        ReviewAcceptanceEventInputV4(
            subject.subject_kind,
            DigestReferenceV1.from_identity(subject.identity()),
            _reviewer_roster(),
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
            (
                AcceptanceEvidenceRefV1(
                    path=f"sources/review-evidence-{marker}.json",
                    raw_sha256=bytes([marker]) * 32,
                    locator=("whole_artifact", None),
                ),
            ),
        )
    )


def _application_record(
    application_id: AuthorityIdentityV1,
    theorem_record_id: AuthorityIdentityV1,
    members: tuple[object, ...],
    marker: int,
) -> tuple[RelationApplicationV2Record, ReviewAcceptanceEventLeafV4]:
    record_shape = RelationApplicationV2Record.from_parts(
        application_id,
        theorem_record_id,
        "required_interaction",
        members,  # type: ignore[arg-type]
        event_ref(),
    )
    subject = AcceptanceSubjectPayloadV4(
        AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD,
        record_shape.acceptance_free_subject_payload(),
    )
    event = _event_for_subject(subject, marker)
    record = RelationApplicationV2Record.from_parts(
        application_id,
        theorem_record_id,
        "required_interaction",
        members,  # type: ignore[arg-type]
        _review_event_ref(event, marker),
    )
    return record, event


def _supersession_record(
    superseded: RelationApplicationV2Record,
    replacement: RelationApplicationV2Record | None,
    reason: SupersessionReason,
    marker: int,
) -> tuple[RelationApplicationV2SupersessionRecord, ReviewAcceptanceEventLeafV4]:
    semantic = RelationApplicationV2SupersessionInputV2(
        superseded.record_id.digest_bytes,
        None if replacement is None else replacement.record_id.digest_bytes,
        None if replacement is None else "relation_application_v2_record",
        reason,
        (evidence(),),
    )
    placeholder = RelationApplicationV2SupersessionRecord.from_parts(
        semantic.identity(),
        superseded.record_id,
        None if replacement is None else replacement.record_id,
        reason,
        (evidence(),),
        event_ref(),
    )
    subject = AcceptanceSubjectPayloadV4(
        AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_SUPERSESSION_RECORD,
        placeholder.acceptance_free_subject_payload(),
    )
    event = _event_for_subject(subject, marker)
    record = RelationApplicationV2SupersessionRecord.from_parts(
        semantic.identity(),
        superseded.record_id,
        None if replacement is None else replacement.record_id,
        reason,
        (evidence(),),
        _review_event_ref(event, marker),
    )
    return record, event


class RelationApplicationV2SupersessionTests(unittest.TestCase):
    def test_currentness_evaluator_conformance_matrix_reports_full_end_state(self) -> None:
        base, _ = valid_record()
        theorem_id = base.theorem_record_id
        application_x = base.application_id
        record_a, event_a = _application_record(
            application_x, theorem_id, base.members, 1
        )
        record_b, event_b = _application_record(
            application_x, theorem_id, base.members, 2
        )
        record_c, event_c = _application_record(
            application_x, theorem_id, base.members, 3
        )
        application_y = RelationApplicationV2(
            theorem_record_id_bytes=theorem_id.digest_bytes,
            terminal_disposition="required_interaction",
            members=(member(digest=b"d" * 32, source_instance_id="si/1"),),
        ).identity()
        record_y, event_y = _application_record(
            application_y, theorem_id, (member(digest=b"d" * 32, source_instance_id="si/1"),), 4
        )

        def evaluate_case(
            records: tuple[RelationApplicationV2Record, ...],
            links: tuple[
                tuple[RelationApplicationV2Record, RelationApplicationV2Record | None, int],
                ...,
            ],
            events: tuple[ReviewAcceptanceEventLeafV4, ...],
        ) -> object:
            supersessions: list[RelationApplicationV2SupersessionRecord] = []
            all_events = list(events)
            for superseded, replacement, marker in links:
                supersession, event = _supersession_record(
                    superseded,
                    replacement,
                    SupersessionReason.AUTHORITY_REVOCATION
                    if replacement is None
                    else SupersessionReason.SOURCE_REVISION,
                    marker,
                )
                supersessions.append(supersession)
                all_events.append(event)
            resolver = MultiEventAdmissionResolver(tuple(all_events))
            return RelationApplicationV2CurrentnessEvaluator(
                resolver,
                currentness=FakeCurrentness(theorem=theorem_record()),
            ).evaluate(records, tuple(supersessions))

        only_a = evaluate_case((record_a,), (), (event_a,))
        self.assertEqual(only_a.current_record_ids, (record_a.record_id,))
        self.assertEqual(only_a.superseded_record_ids, ())
        self.assertEqual(only_a.revoked_record_ids, ())

        a_to_b = evaluate_case(
            (record_a, record_b), ((record_a, record_b, 10),), (event_a, event_b)
        )
        self.assertEqual(a_to_b.current_record_ids, (record_b.record_id,))
        self.assertEqual(a_to_b.superseded_record_ids, (record_a.record_id,))
        self.assertEqual(a_to_b.revoked_record_ids, ())

        a_to_b_to_c = evaluate_case(
            (record_a, record_b, record_c),
            ((record_a, record_b, 11), (record_b, record_c, 12)),
            (event_a, event_b, event_c),
        )
        self.assertEqual(a_to_b_to_c.current_record_ids, (record_c.record_id,))
        self.assertEqual(
            a_to_b_to_c.superseded_record_ids,
            tuple(
                sorted((record_a.record_id, record_b.record_id), key=lambda item: item.to_cbor())
            ),
        )

        cross_application = evaluate_case(
            (record_a, record_y), ((record_a, record_y, 13),), (event_a, event_y)
        )
        self.assertEqual(cross_application.current_record_ids, (record_y.record_id,))
        self.assertEqual(cross_application.superseded_record_ids, (record_a.record_id,))

        revoked_group = evaluate_case(
            (record_a, record_b), ((record_a, None, 14),), (event_a, event_b)
        )
        self.assertEqual(revoked_group.current_record_ids, ())
        self.assertEqual(
            revoked_group.revoked_record_ids,
            tuple(
                sorted((record_a.record_id, record_b.record_id), key=lambda item: item.to_cbor())
            ),
        )

        revoked_group_with_other_application = evaluate_case(
            (record_a, record_b, record_y),
            ((record_a, None, 15),),
            (event_a, event_b, event_y),
        )
        self.assertEqual(
            revoked_group_with_other_application.current_record_ids,
            (record_y.record_id,),
        )
        self.assertEqual(
            revoked_group_with_other_application.revoked_record_ids,
            tuple(
                sorted((record_a.record_id, record_b.record_id), key=lambda item: item.to_cbor())
            ),
        )

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
