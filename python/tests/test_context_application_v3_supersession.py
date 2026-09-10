from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from context_application_v3_supersession import (
    ContextApplicationV3CurrentnessError,
    ContextApplicationV3CurrentnessEvaluator,
    ContextApplicationV3SupersessionAdmissionValidator,
    admit_context_application_v3_supersession_record,
)
from mtgml.authority import (
    AcceptanceSubjectKindV4,
    AcceptanceSubjectPayloadV4,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationV3InputV1,
    ContextApplicationV3Record,
    ContextApplicationV3SupersessionInputV1,
    ContextApplicationV3SupersessionRecord,
    ContextApplicationV3SupersessionRecordInputV1,
    DigestReferenceV1,
    EvidenceRefV1,
    ReviewAcceptanceEventInputV4,
    ReviewAcceptanceEventLeafV4,
    ReviewerRoleBindingV1,
    ReviewEventRefV4,
    ReviewMode,
    SupersessionReason,
)
from mtgml.persistence import encode_canonical
from test_context_application_v3_contract import member
from test_review_acceptance_event_v4 import (
    REQUIRED_ROLES,
)
from test_review_acceptance_event_v4 import (
    evidence as v4_evidence,
)
from test_review_acceptance_event_v4 import (
    roster_ref as v4_roster_ref,
)
from test_review_acceptance_event_v4 import (
    source_bindings as v4_source_bindings,
)


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
    @staticmethod
    def _admitted(value: ContextApplicationV3SupersessionRecord) -> object:
        return SimpleNamespace(
            event_id=value.review_event_ref_v4.event_id,
            exact_event_closure=(value.review_event_ref_v4,),
        )

    def test_cps_cpsr_identity_vectors_match_frozen_fixture(self) -> None:
        matrix = json.loads(
            (
                ROOT / "conformance/fixtures/authority/"
                "context_application_v3_supersession_identity_golden_matrix.v1.json"
            ).read_text(encoding="utf-8")
        )
        source = record("a" * 64)
        evidence = EvidenceRefV1("model", "sources/model.json", ("whole_artifact", None), b"e" * 32)
        supersession = ContextApplicationV3SupersessionInputV1(
            source.record_id.digest_bytes,
            None,
            None,
            SupersessionReason.AUTHORITY_REVOCATION,
            (evidence,),
        )
        inputs = {
            "context_supersession_v3": supersession,
            "context_supersession_record_v3": ContextApplicationV3SupersessionRecordInputV1(
                supersession.identity().digest_bytes,
                source.review_event_ref_v4,
            ),
        }
        for entry in matrix["identities"]:
            value = inputs[entry["kind"]]
            identity = value.identity()
            self.assertEqual(identity.as_text(), entry["identity"])
            self.assertEqual(encode_canonical(identity.to_cbor()).hex(), entry["identity_cbor_hex"])

    def test_currentness_requires_record_and_supersession_admission(self) -> None:
        first = record("a" * 64)
        with self.assertRaises(TypeError):
            ContextApplicationV3CurrentnessEvaluator().evaluate((first,), ())

    def test_application_admission_errors_are_not_silently_dropped(self) -> None:
        first = record("a" * 64)

        class StaleTheorem(ValueError):
            code = "SUPERSEDED_AUTHORITY_USED"

        def reject(_value: object) -> object:
            raise StaleTheorem("stale theorem")

        with self.assertRaises(ContextApplicationV3CurrentnessError) as raised:
            ContextApplicationV3CurrentnessEvaluator(
                record_admitter=reject,
                supersession_admitter=ContextApplicationV3SupersessionAdmissionValidator(
                    own_record_admitter=self._admitted
                ),
            ).evaluate((first,), ())
        self.assertEqual(raised.exception.code, "CURRENTNESS_RECORD_ADMISSION_FAILED")
        self.assertEqual(raised.exception.cause_code, "SUPERSEDED_AUTHORITY_USED")

    def test_supersession_requires_own_v4_admission(self) -> None:
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
        with self.assertRaises(TypeError):
            ContextApplicationV3SupersessionAdmissionValidator().admit(supersession)
        with self.assertRaisesRegex(Exception, "REVIEW_ADMISSION_FAILED"):
            ContextApplicationV3SupersessionAdmissionValidator(
                own_record_admitter=lambda value: value
            ).admit(supersession)

    def test_cpsr_admission_uses_the_real_generic_v4_binder(self) -> None:
        endpoint = record("a" * 64)
        evidence = EvidenceRefV1("model", "sources/model.json", ("whole_artifact", None), b"e" * 32)
        supersession_id = ContextApplicationV3SupersessionInputV1(
            endpoint.record_id.digest_bytes,
            None,
            None,
            SupersessionReason.AUTHORITY_REVOCATION,
            (evidence,),
        ).identity()
        placeholder_ref = ReviewEventRefV4(
            "sources/m2_5/authorities/review_acceptance_events/v4/" + "a" * 64 + ".json",
            b"a" * 32,
            "ae.v4/" + "a" * 64,
        )
        provisional = ContextApplicationV3SupersessionRecord.from_parts(
            supersession_id,
            endpoint.record_id,
            None,
            SupersessionReason.AUTHORITY_REVOCATION,
            (evidence,),
            placeholder_ref,
        )
        subject = AcceptanceSubjectPayloadV4(
            AcceptanceSubjectKindV4.CONTEXT_APPLICATION_V3_SUPERSESSION_RECORD,
            provisional.acceptance_free_subject_payload(),
        )
        event = ReviewAcceptanceEventLeafV4.from_input(
            ReviewAcceptanceEventInputV4(
                AcceptanceSubjectKindV4.CONTEXT_APPLICATION_V3_SUPERSESSION_RECORD,
                DigestReferenceV1.from_identity(subject.identity()),
                v4_roster_ref(),
                (ReviewerRoleBindingV1("reviewer", REQUIRED_ROLES),),
                ReviewMode.MULTI_REVIEWER,
                v4_source_bindings(),
                (v4_evidence(),),
            )
        )
        event_ref = ReviewEventRefV4(
            "sources/m2_5/authorities/review_acceptance_events/v4/"
            + event.event_id.digest_bytes.hex()
            + ".json",
            event.event_id.digest_bytes,
            event.event_id.as_text(),
        )
        record_value = ContextApplicationV3SupersessionRecord.from_parts(
            supersession_id,
            endpoint.record_id,
            None,
            SupersessionReason.AUTHORITY_REVOCATION,
            (evidence,),
            event_ref,
        )

        class Resolver:
            def resolve_acceptance_event_leaf_v4(
                self, _reference: ReviewEventRefV4
            ) -> ReviewAcceptanceEventLeafV4:
                return event

            def expected_context_application_v3_supersession_source_closure(
                self, _record: object, _roster: object
            ) -> tuple[object, ...]:
                return v4_source_bindings()

            def resolve_v4_source_binding(self, _binding: object) -> object:
                return object()

            def resolve_v4_acceptance_evidence(self, _evidence: object) -> object:
                return object()

        admitted = admit_context_application_v3_supersession_record(record_value, Resolver())
        self.assertEqual(admitted.event_id, event.event_id.as_text())
        self.assertEqual(len(admitted.exact_event_closure), len(v4_source_bindings()))

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
        admitted = ContextApplicationV3SupersessionAdmissionValidator(
            own_record_admitter=self._admitted
        ).admit(supersession)
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
        result = ContextApplicationV3CurrentnessEvaluator(
            record_admitter=lambda value: value,
            supersession_admitter=ContextApplicationV3SupersessionAdmissionValidator(
                own_record_admitter=self._admitted
            ),
        ).evaluate((first, second), (supersession,))
        self.assertEqual(result.current_record_ids, (second.record_id,))
        self.assertEqual(result.superseded_record_ids, (first.record_id,))


if __name__ == "__main__":
    unittest.main()
