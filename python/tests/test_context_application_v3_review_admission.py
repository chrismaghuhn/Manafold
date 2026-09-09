from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from context_application_v3_review_admission import (
    ContextApplicationV3ReviewAdmissionError,
    admit_context_application_v3_record,
)
from mtgml.authority import ReviewEventRefV4
from test_context_application_v3_review_binding import (
    FakeV3ReviewResolver,
    make_event,
    make_record,
)
from test_context_application_v3_validator import FakeV3Resolver


class AdmissionResolver(FakeV3Resolver, FakeV3ReviewResolver):
    def __init__(self, event: object) -> None:
        from test_relation_application_v2_contract import member as rpa_member

        FakeV3Resolver.__init__(self, rpa_member())
        self.event = event

    def resolve_acceptance_event_leaf_v4(self, _reference: ReviewEventRefV4) -> object:
        return self.event

    def resolve_v4_source_binding(self, _binding: object) -> object:
        return object()

    def resolve_v4_acceptance_evidence(self, _evidence: object) -> object:
        return object()

    def expected_context_application_v3_source_closure(
        self, _record: object, _roster: object
    ) -> tuple[object, ...]:
        from test_review_acceptance_event_v4 import source_bindings

        return source_bindings()


class ContextApplicationV3ReviewAdmissionTests(unittest.TestCase):
    def _record_and_resolver(self) -> tuple[object, AdmissionResolver]:
        provisional = make_record()
        event = make_event(provisional)
        record = type(provisional).from_parts(
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
        return record, AdmissionResolver(event)

    def test_record_requires_semantics_and_v4_review_admission(self) -> None:
        record, resolver = self._record_and_resolver()
        result = admit_context_application_v3_record(record, resolver)  # type: ignore[arg-type]
        self.assertEqual(result.record_id, record.record_id.as_text())
        self.assertTrue(result.semantic_validation.valid)

    def test_invalid_record_is_rejected_before_review_binding(self) -> None:
        record, resolver = self._record_and_resolver()
        with self.assertRaises(ContextApplicationV3ReviewAdmissionError) as raised:
            admit_context_application_v3_record(object(), resolver)  # type: ignore[arg-type]
        self.assertEqual(raised.exception.code, "APPLICATION_INPUT_INVALID")


if __name__ == "__main__":
    unittest.main()
