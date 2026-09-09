from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from context_application_v3_review_binding import (
    ContextApplicationV3ReviewBindingError,
    bind_context_application_v3_review,
)
from mtgml.authority import (
    AcceptanceSubjectKindV4,
    AcceptanceSubjectPayloadV4,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationV3InputV1,
    ContextApplicationV3Record,
    DigestReferenceV1,
    ReviewAcceptanceEventInputV4,
    ReviewAcceptanceEventLeafV4,
    ReviewerRoleBindingV1,
    ReviewEventRefV4,
    ReviewMode,
)
from test_context_application_v3_contract import event_ref, member
from test_relation_application_v2_contract import member as rpa_member
from test_review_acceptance_event_v4 import REQUIRED_ROLES, evidence, roster_ref, source_bindings


class FakeV3ReviewResolver:
    def __init__(self, event: ReviewAcceptanceEventLeafV4) -> None:
        self.event = event

    def resolve_acceptance_event_leaf_v4(
        self, _reference: ReviewEventRefV4
    ) -> ReviewAcceptanceEventLeafV4:
        return self.event

    def resolve_v4_source_binding(self, _binding: object) -> object:
        return object()

    def resolve_v4_acceptance_evidence(self, _evidence: object) -> object:
        return object()

    def expected_context_application_v3_source_closure(
        self, _record: ContextApplicationV3Record, _roster: object
    ) -> tuple[object, ...]:
        return source_bindings()


def make_record() -> ContextApplicationV3Record:
    application = ContextApplicationV3InputV1(b"t" * 32, (member(),))
    return ContextApplicationV3Record.from_parts(
        application.identity(),
        AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_THEOREM_RECORD, b"t" * 32),
        application.members,
        event_ref(),
    )


def make_event(record: ContextApplicationV3Record) -> ReviewAcceptanceEventLeafV4:
    subject = AcceptanceSubjectPayloadV4(
        AcceptanceSubjectKindV4.CONTEXT_APPLICATION_V3_RECORD,
        record.acceptance_free_subject_payload(),
    )
    return ReviewAcceptanceEventLeafV4.from_input(
        ReviewAcceptanceEventInputV4(
            AcceptanceSubjectKindV4.CONTEXT_APPLICATION_V3_RECORD,
            DigestReferenceV1.from_identity(subject.identity()),
            roster_ref(),
            (ReviewerRoleBindingV1("reviewer", REQUIRED_ROLES),),
            ReviewMode.MULTI_REVIEWER,
            source_bindings(),
            (evidence(),),
        )
    )


class ContextApplicationV3ReviewBindingTests(unittest.TestCase):
    def test_valid_v3_record_binds_v4_subject_and_closure(self) -> None:
        provisional = make_record()
        event = make_event(provisional)
        record = ContextApplicationV3Record.from_parts(
            provisional.application_id,
            provisional.theorem_record_id,
            provisional.members,
            ReviewEventRefV4(
                "sources/m2_5/authorities/review_acceptance_events/v4/"
                + event.event_id.digest_bytes.hex()
                + ".json",
                b"a" * 32,
                event.event_id.as_text(),
            ),
        )
        result = bind_context_application_v3_review(record, FakeV3ReviewResolver(event))
        self.assertEqual(result.record_id, record.record_id.as_text())
        self.assertEqual(
            result.subject_digest_reference.digest_bytes,
            event.subject_payload_digest_reference.digest_bytes,
        )

    def test_wrong_subject_kind_fails_closed(self) -> None:
        record = make_record()
        event = ReviewAcceptanceEventLeafV4.from_input(
            ReviewAcceptanceEventInputV4(
                AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD,
                DigestReferenceV1.from_identity(
                    AcceptanceSubjectPayloadV4(
                        AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD,
                        [
                            "relation_application_v2_record",
                            b"a" * 32,
                            b"b" * 32,
                            "required_interaction",
                            [rpa_member().to_cbor()],
                        ],
                    ).identity()
                ),
                roster_ref(),
                (ReviewerRoleBindingV1("reviewer", REQUIRED_ROLES),),
                ReviewMode.MULTI_REVIEWER,
                source_bindings(),
                (evidence(),),
            )
        )
        with self.assertRaises(ContextApplicationV3ReviewBindingError) as raised:
            bind_context_application_v3_review(record, FakeV3ReviewResolver(event))
        self.assertEqual(raised.exception.code, "CONTEXT_APPLICATION_V3_SUBJECT_KIND_MISMATCH")


if __name__ == "__main__":
    unittest.main()
