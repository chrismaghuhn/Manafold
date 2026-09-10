from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from context_application_v3_supersession import (
    ContextApplicationV3CurrentnessEvaluator,
    ContextApplicationV3SupersessionAdmissionValidator,
)
from mtgml.authority import (
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationV3InputV1,
    ContextApplicationV3Record,
    ContextApplicationV3SupersessionInputV1,
    ContextApplicationV3SupersessionRecord,
    EvidenceRefV1,
    ReviewEventRefV4,
    SupersessionReason,
)
from test_context_application_v3_contract import member


def record(event_hex: str) -> ContextApplicationV3Record:
    application = ContextApplicationV3InputV1(b"t" * 32, (member(),))
    return ContextApplicationV3Record.from_parts(
        application.identity(),
        AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_THEOREM_RECORD, b"t" * 32),
        application.members,
        ReviewEventRefV4(
            "sources/m2_5/authorities/review_acceptance_events/v4/" + event_hex + ".json",
            bytes.fromhex(event_hex),
            "ae.v4/" + event_hex,
        ),
    )


class ContextApplicationV3SupersessionTests(unittest.TestCase):
    def test_cps_and_cpsr_v3_are_versioned_and_revocation_is_closed(self) -> None:
        source = record("a" * 64)
        supersession_id = ContextApplicationV3SupersessionInputV1(
            source.record_id.digest_bytes,
            None,
            None,
            SupersessionReason.AUTHORITY_REVOCATION,
            (EvidenceRefV1("model", "sources/model.json", ("whole_artifact", None), b"e" * 32),),
        ).identity()
        supersession = ContextApplicationV3SupersessionRecord.from_parts(
            supersession_id,
            source.record_id,
            None,
            SupersessionReason.AUTHORITY_REVOCATION,
            (EvidenceRefV1("model", "sources/model.json", ("whole_artifact", None), b"e" * 32),),
            source.review_event_ref_v4,
        )
        self.assertEqual(
            supersession.supersession_id.kind, AuthorityIdentityKind.CONTEXT_SUPERSESSION_V3
        )
        self.assertEqual(
            supersession.record_id.kind, AuthorityIdentityKind.CONTEXT_SUPERSESSION_RECORD_V3
        )
        admitted = ContextApplicationV3SupersessionAdmissionValidator().admit(supersession)
        self.assertEqual(admitted.record_id, supersession.record_id)

    def test_a_to_b_excludes_a_and_keeps_b_current(self) -> None:
        first = record("a" * 64)
        second = record("b" * 64)
        supersession_id = ContextApplicationV3SupersessionInputV1(
            first.record_id.digest_bytes,
            second.record_id.digest_bytes,
            "context_application_v3_record",
            SupersessionReason.SEMANTIC_CORRECTION,
            (EvidenceRefV1("model", "sources/model.json", ("whole_artifact", None), b"e" * 32),),
        ).identity()
        supersession = ContextApplicationV3SupersessionRecord.from_parts(
            supersession_id,
            first.record_id,
            second.record_id,
            SupersessionReason.SEMANTIC_CORRECTION,
            (EvidenceRefV1("model", "sources/model.json", ("whole_artifact", None), b"e" * 32),),
            first.review_event_ref_v4,
        )
        result = ContextApplicationV3CurrentnessEvaluator().evaluate(
            (first, second), (supersession,)
        )
        self.assertEqual(result.current_record_ids, (second.record_id,))
        self.assertEqual(result.superseded_record_ids, (first.record_id,))


if __name__ == "__main__":
    unittest.main()
