from __future__ import annotations

import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from context_application_v3_resolver import (
    ContextApplicationV3ResolutionError,
    ContextApplicationV3Resolver,
    ContextApplicationV3RpaResolver,
    ResolvedRpaV2Member,
)
from mtgml.authority import (
    AcceptanceSubjectKindV4,
    AcceptanceSubjectPayloadV4,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationV3InputV1,
    ContextApplicationV3Record,
    DigestReferenceV1,
    RelationApplicationAuthorityV2,
    RelationAuthoritySourceBindingV2,
    ReviewAcceptanceEventInputV4,
    ReviewAcceptanceEventLeafV4,
    ReviewAuthoritySourceBindingV4,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewEventRefV4,
    ReviewMode,
)
from test_context_application_v3_contract import member
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
    def _resolver(self, status: str = "current") -> tuple[ContextApplicationV3RpaResolver, object]:
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
            currentness=FakeCurrentness(status, theorem_record()),
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

    def test_stale_revoked_and_ambiguous_rpa_currentness_fail_closed(self) -> None:
        for status in ("superseded", "revoked", "ambiguous"):
            with self.subTest(status=status):
                with self.assertRaises(ContextApplicationV3ResolutionError) as raised:
                    self._resolver(status)
                self.assertEqual(raised.exception.code, "CONTEXT_APPLICATION_V3_RPA_NOT_CURRENT")

    def test_context_closure_uses_rpa_event_roster_for_transitive_rpa_sources(self) -> None:
        rpa_record, _ = valid_record()
        context_application = ContextApplicationV3InputV1(b"t" * 32, (member(),))
        context_record = ContextApplicationV3Record.from_parts(
            context_application.identity(),
            AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_THEOREM_RECORD, b"t" * 32),
            context_application.members,
            ReviewEventRefV4(
                "sources/m2_5/authorities/review_acceptance_events/v4/" + "b" * 64 + ".json",
                b"b" * 32,
                "ae.v4/" + "b" * 64,
            ),
        )
        context_roster = ReviewerRosterRefV1(
            "sources/m2_5/authorities/reviewer_rosters/v1/" + "73" * 32 + ".json",
            "manafold.m2.5.c.reviewer-roster.v1",
            bytes.fromhex("73" * 32),
        )
        rpa_roster = ReviewerRosterRefV1(
            "sources/m2_5/authorities/reviewer_rosters/v1/" + "74" * 32 + ".json",
            "manafold.m2.5.c.reviewer-roster.v1",
            bytes.fromhex("74" * 32),
        )
        captured: list[ReviewerRosterRefV1] = []

        class RpaMemberResolver:
            def resolve_current_rpa_member(self, *_args: object) -> ResolvedRpaV2Member:
                return ResolvedRpaV2Member(rpa_record, object())

            def expected_rpa_source_closure(
                self, _record: object, reviewer_roster_ref: ReviewerRosterRefV1
            ) -> tuple[ReviewAuthoritySourceBindingV4, ...]:
                captured.append(reviewer_roster_ref)
                return (
                    ReviewAuthoritySourceBindingV4(
                        "reviewer_roster_leaf",
                        reviewer_roster_ref.path,
                        reviewer_roster_ref.schema,
                        reviewer_roster_ref.raw_sha256,
                    ),
                )

        resolver = object.__new__(ContextApplicationV3Resolver)
        resolver._base_binding = SimpleNamespace()
        resolver._rpa_member_resolver = RpaMemberResolver()
        resolver._v2_resolver = SimpleNamespace(
            _base_context=lambda _binding: (
                object(),
                SimpleNamespace(
                    path="sources/m2_5/closures/C/declared_interaction_model.v2.json",
                    schema="manafold.m2.5.c.declared-interaction-model.v2",
                    raw_sha256=b"m" * 32,
                ),
                (),
                (),
            ),
            _candidate_provenance=lambda _member: ((), ()),
            _member_evidence=lambda _member: (),
            _collect_evidence=lambda _evidence: ((), set(), False),
            _walk_v1_dependencies=lambda _theorem: ((), set(), False),
        )
        resolver._validated_base = lambda: (object(), {})
        resolver.resolve_current_context_theorem = lambda _theorem_id: {}
        resolver.resolve_acceptance_event_leaf_v4 = lambda _reference: SimpleNamespace(
            reviewer_roster_ref=rpa_roster
        )
        resolver._review_event_ref_v1 = lambda _theorem: object()
        resolver._v1_event_closure = lambda _reference: []
        with patch(
            "context_application_v3_resolver.reconstruct_event_source_closure",
            return_value=(),
        ):
            closure = resolver.expected_context_application_v3_source_closure(
                context_record, context_roster
            )

        self.assertEqual(captured, [rpa_roster])
        self.assertIn(rpa_roster.raw_sha256, {item.raw_sha256 for item in closure})
        self.assertIn(context_roster.raw_sha256, {item.raw_sha256 for item in closure})


if __name__ == "__main__":
    unittest.main()
