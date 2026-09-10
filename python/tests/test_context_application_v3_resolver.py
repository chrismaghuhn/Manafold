from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from context_application_v3_resolver import (
    ContextApplicationV3ResolutionError,
    ContextApplicationV3RpaResolver,
)
from mtgml.authority import (
    AcceptanceSubjectKindV4,
    AcceptanceSubjectPayloadV4,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    DigestReferenceV1,
    RelationApplicationAuthorityV2,
    RelationAuthoritySourceBindingV2,
    ReviewAcceptanceEventInputV4,
    ReviewAcceptanceEventLeafV4,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewMode,
)
from test_relation_application_v2_review_admission import (
    FakeAdmissionResolver,
    FakeCurrentness,
    review_evidence,
    source_binding,
    theorem_record,
    valid_record,
)


class RpaClosureResolver(FakeAdmissionResolver):
    def validate_relation_application_authority_v2_source_closure(
        self, authority: RelationApplicationAuthorityV2
    ) -> tuple[RelationAuthoritySourceBindingV2, ...]:
        return authority.source_bindings

    def expected_relation_application_v2_source_closure(
        self, _record: object, _reviewer_roster_ref: object
    ) -> tuple[object, ...]:
        return tuple(self.event.source_binding_digests)


def event_for_record(record: object) -> ReviewAcceptanceEventLeafV4:
    subject = AcceptanceSubjectPayloadV4(
        AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD,
        record.acceptance_free_subject_payload(),
    )
    return ReviewAcceptanceEventLeafV4.from_input(
        ReviewAcceptanceEventInputV4(
            AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD,
            DigestReferenceV1.from_identity(subject.identity()),
            ReviewerRosterRefV1(
                "sources/m2_5/authorities/reviewer_rosters/v1/" + "72" * 32 + ".json",
                "manafold.m2.5.c.reviewer-roster.v1",
                bytes.fromhex("72" * 32),
            ),
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


class ContextApplicationV3ResolverTests(unittest.TestCase):
    def _resolver(self) -> tuple[ContextApplicationV3RpaResolver, object]:
        record, _ = valid_record()
        base = RelationAuthoritySourceBindingV2(
            "base_authority_v1",
            "sources/m2_5/authorities/interaction_review_authority.v1.json",
            "manafold.m2.5.c.interaction-review-authority.v1",
            b"a" * 32,
        )
        candidate = RelationAuthoritySourceBindingV2(
            "candidate_universe",
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            "manafold.m2.5.c.interaction-candidate-universe.v2",
            b"b" * 32,
        )
        authority = RelationApplicationAuthorityV2(
            base,
            candidate,
            (base, candidate),
            (record,),
            (),
        )
        admission_resolver = RpaClosureResolver(event_for_record(record))
        resolver = ContextApplicationV3RpaResolver(
            authority,
            admission_resolver,
            currentness=FakeCurrentness(theorem=theorem_record()),
        )
        return resolver, record

    def test_rpa_transitive_closure_is_a_required_typed_dependency(self) -> None:
        resolver, record = self._resolver()
        closure = resolver.expected_rpa_source_closure(record, object())
        self.assertTrue(closure)

    def test_exact_current_rpa_member_is_resolved(self) -> None:
        resolver, record = self._resolver()
        resolved = resolver.resolve_current_rpa_member(
            record.application_id,
            record.members[0].candidate_id,
            record.members[0].candidate_identity_digest_reference,
            record.members[0].source_instance_id,
        )
        self.assertEqual(resolved.record.record_id, record.record_id)

    def test_unknown_rpa_application_fails_closed(self) -> None:
        resolver, record = self._resolver()
        unknown = AuthorityIdentityV1(AuthorityIdentityKind.RELATION_APPLICATION_V2, b"z" * 32)
        with self.assertRaises(ContextApplicationV3ResolutionError) as raised:
            resolver.resolve_current_rpa_member(
                unknown,
                record.members[0].candidate_id,
                record.members[0].candidate_identity_digest_reference,
                record.members[0].source_instance_id,
            )
        self.assertEqual(raised.exception.code, "CONTEXT_APPLICATION_V3_RPA_NOT_CURRENT")


if __name__ == "__main__":
    unittest.main()
