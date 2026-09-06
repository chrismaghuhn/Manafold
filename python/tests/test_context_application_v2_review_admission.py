from __future__ import annotations

import hashlib
import json
import sys
import unittest
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from authority_source_resolver import AuthoritySourceResolver
from context_application_v2_resolver import ContextApplicationV2Resolver
from mtgml.authority import (
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKindV3,
    AcceptanceSubjectPayloadV3,
    ContextApplicationV2Record,
    DigestReferenceV1,
    ReviewAcceptanceEventInputV3,
    ReviewAcceptanceEventLeafV3,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewEventRefV3,
    ReviewMode,
)


class ContextApplicationV2ReviewAdmissionTests(unittest.TestCase):
    def _synthetic_case(
        self,
        *,
        source_timing: str = "not_applicable",
        reviewed_timing: str = "not_applicable",
        source_visibility: str = "not_applicable",
        reviewed_visibility: str | None = None,
    ) -> dict[str, object]:
        from test_context_application_v2_validator import ContextApplicationV2IntegrationTests

        base = ContextApplicationV2IntegrationTests()
        base.setUp()
        self.addCleanup(base.doCleanups)
        self.addCleanup(base.tearDown)
        return base._synthetic_case(
            source_timing=source_timing,
            reviewed_timing=reviewed_timing,
            source_visibility=source_visibility,
            reviewed_visibility=reviewed_visibility,
        )

    def _record_with_v3_event(
        self,
        case: dict[str, object],
        *,
        review_mode: ReviewMode = ReviewMode.MULTI_REVIEWER,
    ) -> tuple[object, ContextApplicationV2Record, dict[str, object]]:
        fixture = case["fixture"]
        repo = cast(Path, fixture.repo)
        source_resolver = cast(object, case["source_resolver"])
        base_binding = case["base_binding"]
        initial_record = cast(ContextApplicationV2Record, case["record"])

        roster_raw = json.dumps(
            {
                "schema": "manafold.m2.5.c.reviewer-roster.v1",
                "reviewers": [
                    {
                        "reviewer_id": "alice",
                        "roles": [
                            "architecture_maintainer",
                            "conformance_maintainer",
                            "information_safety_reviewer",
                            "rules_authority_maintainer",
                        ],
                    }
                ],
            },
            separators=(",", ":"),
        ).encode("utf-8")
        roster_digest = hashlib.sha256(roster_raw).digest()
        roster_path = (
            "sources/m2_5/authorities/reviewer_rosters/v1/"
            + roster_digest.hex()
            + ".json"
        )
        fixture.write_repo(roster_path, roster_raw)
        roster_ref = ReviewerRosterRefV1(
            roster_path,
            "manafold.m2.5.c.reviewer-roster.v1",
            roster_digest,
        )

        evidence_path = "docs/review/context-application-v2-slice4.md"
        evidence_raw = b"synthetic Slice-4 review evidence\n"
        fixture.write_repo(evidence_path, evidence_raw)
        evidence_ref = AcceptanceEvidenceRefV1(
            evidence_path,
            hashlib.sha256(evidence_raw).digest(),
            ("whole_artifact", None),
        )

        resolver = ContextApplicationV2Resolver(
            cast(AuthoritySourceResolver, source_resolver),
            base_authority_binding=base_binding,
        )
        provisional = ContextApplicationV2Record.from_parts(
            application_id=initial_record.application_id,
            theorem_record_id=initial_record.theorem_record_id,
            members=initial_record.members,
            review_event_ref_v3=ReviewEventRefV3(
                "sources/m2_5/authorities/review_acceptance_events/v3/"
                + "00" * 32
                + ".json",
                bytes(32),
                "ae.v3/" + "00" * 32,
            ),
        )
        closure = resolver.expected_acceptance_source_closure_v3(provisional, roster_ref)
        subject = AcceptanceSubjectPayloadV3(
            subject_kind=AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD,
            subject_payload=provisional.acceptance_free_subject_payload(),
        )
        event_input = ReviewAcceptanceEventInputV3(
            subject_kind=AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD,
            subject_payload_digest_reference=DigestReferenceV1.from_identity(subject.identity()),
            reviewer_roster_ref=roster_ref,
            reviewer_role_bindings=(
                ReviewerRoleBindingV1(
                    "alice",
                    (
                        "architecture_maintainer",
                        "conformance_maintainer",
                        "information_safety_reviewer",
                        "rules_authority_maintainer",
                    ),
                ),
            ),
            review_mode=review_mode,
            source_binding_digests=closure,
            review_evidence_refs=(evidence_ref,),
        )
        event_wire = ReviewAcceptanceEventLeafV3.from_input(event_input).to_wire()
        event_raw = (json.dumps(event_wire, separators=(",", ":")) + "\n").encode("utf-8")
        event_id = cast(str, event_wire["event_id"])
        event_path = (
            "sources/m2_5/authorities/review_acceptance_events/v3/"
            + event_id.removeprefix("ae.v3/")
            + ".json"
        )
        fixture.write_repo(event_path, event_raw)
        event_ref = ReviewEventRefV3(event_path, hashlib.sha256(event_raw).digest(), event_id)
        record = ContextApplicationV2Record.from_parts(
            application_id=initial_record.application_id,
            theorem_record_id=initial_record.theorem_record_id,
            members=initial_record.members,
            review_event_ref_v3=event_ref,
        )
        return source_resolver, record, event_wire

    def test_exact_match_application_is_admitted(self) -> None:
        from context_application_v2_review_admission import (
            ContextApplicationV2ReviewAdmissionResult,
            ContextApplicationV2ReviewAdmissionValidator,
        )

        case = self._synthetic_case()
        source_resolver, record, _ = self._record_with_v3_event(case)
        result = ContextApplicationV2ReviewAdmissionValidator(
            source_resolver,
            base_authority_binding=case["base_binding"],
        ).admit(record)
        self.assertIsInstance(result, ContextApplicationV2ReviewAdmissionResult)

    def test_reviewed_divergence_application_is_admitted(self) -> None:
        from context_application_v2_review_admission import (
            ContextApplicationV2ReviewAdmissionValidator,
        )

        case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="activation_time",
        )
        source_resolver, record, _ = self._record_with_v3_event(case)
        ContextApplicationV2ReviewAdmissionValidator(
            source_resolver,
            base_authority_binding=case["base_binding"],
        ).admit(record)

    def test_information_sensitive_and_solo_modes_have_mechanical_positive_controls(self) -> None:
        from context_application_v2_review_admission import (
            ContextApplicationV2ReviewAdmissionValidator,
        )

        case = self._synthetic_case(
            source_visibility="private",
            reviewed_visibility="private",
        )
        source_resolver, record, _ = self._record_with_v3_event(
            case,
            review_mode=ReviewMode.SOLO_SEPARATE_SELF_REVIEW,
        )
        result = ContextApplicationV2ReviewAdmissionValidator(
            source_resolver,
            base_authority_binding=case["base_binding"],
        ).admit(record)
        self.assertEqual(result.review_mode, ReviewMode.SOLO_SEPARATE_SELF_REVIEW)

    def test_result_surface_is_frozen_and_does_not_expose_artifacts(self) -> None:
        import dataclasses

        from context_application_v2_review_admission import (
            ContextApplicationV2ReviewAdmissionValidator,
        )

        case = self._synthetic_case()
        source_resolver, record, _ = self._record_with_v3_event(case)
        result = ContextApplicationV2ReviewAdmissionValidator(
            source_resolver,
            base_authority_binding=case["base_binding"],
        ).admit(record)
        self.assertTrue(dataclasses.is_dataclass(result))
        self.assertIsInstance(result.exact_event_closure, tuple)
        self.assertFalse(hasattr(result, "artifact"))
        self.assertFalse(hasattr(result, "resolved_event"))
        with self.assertRaises(dataclasses.FrozenInstanceError):
            result.review_mode = ReviewMode.MULTI_REVIEWER  # type: ignore[misc]


if __name__ == "__main__":
    unittest.main()
