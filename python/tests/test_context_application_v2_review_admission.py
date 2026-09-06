from __future__ import annotations

import copy
import hashlib
import json
import sys
import unittest
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from authority_source_resolver import AuthoritySourceResolver
from context_application_v2_resolver import (
    ContextApplicationV2Resolver,
    context_source_binding_from_wire,
)
from mtgml.authority import (
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKindV3,
    AcceptanceSubjectPayloadV3,
    ContextApplicationV2Record,
    ContextAuthoritySourceBindingV2,
    DigestReferenceV1,
    ReviewAcceptanceEventInputV3,
    ReviewAcceptanceEventLeafV3,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewEventRefV3,
    ReviewMode,
)
from mtgml.persistence import encode_canonical


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
        reviewer_roles: tuple[str, ...] = (
            "architecture_maintainer",
            "conformance_maintainer",
            "information_safety_reviewer",
            "rules_authority_maintainer",
        ),
    ) -> tuple[object, ContextApplicationV2Record, dict[str, object]]:
        reviewer_roles = tuple(sorted(reviewer_roles))
        fixture = case["fixture"]
        source_resolver = cast(object, case["source_resolver"])
        base_binding = case["base_binding"]
        initial_record = cast(ContextApplicationV2Record, case["record"])

        roster_raw = json.dumps(
            {
                "schema": "manafold.m2.5.c.reviewer-roster.v1",
                "reviewers": [
                    {
                        "reviewer_id": "alice",
                        "roles": list(reviewer_roles),
                    }
                ],
            },
            separators=(",", ":"),
        ).encode("utf-8")
        roster_digest = hashlib.sha256(roster_raw).digest()
        roster_path = (
            "sources/m2_5/authorities/reviewer_rosters/v1/" + roster_digest.hex() + ".json"
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
                "sources/m2_5/authorities/review_acceptance_events/v3/" + "00" * 32 + ".json",
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
                    reviewer_roles,
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

    @staticmethod
    def _digest_reference_from_wire(value: object) -> DigestReferenceV1:
        record = cast(dict[str, object], value)
        return DigestReferenceV1(
            cast(str, record["envelope_id"]),
            cast(str, record["algorithm_id"]),
            cast(str, record["semantic_domain"]),
            cast(str, record["payload_codec_id"]),
            cast(str, record["input_schema_id"]),
            bytes.fromhex(cast(str, record["digest_hex"])),
        )

    @classmethod
    def _event_input_from_wire(cls, wire: dict[str, object]) -> ReviewAcceptanceEventInputV3:
        roster_wire = cast(dict[str, object], wire["reviewer_roster_ref"])
        bindings = tuple(
            ReviewerRoleBindingV1(
                cast(str, cast(dict[str, object], item)["reviewer_id"]),
                tuple(cast(list[str], cast(dict[str, object], item)["roles"])),
            )
            for item in cast(list[object], wire["reviewer_role_bindings"])
        )
        sources = tuple(
            context_source_binding_from_wire(item)
            for item in cast(list[object], wire["source_binding_digests"])
        )
        evidence = tuple(
            AcceptanceEvidenceRefV1(
                cast(str, cast(dict[str, object], item)["path"]),
                bytes.fromhex(cast(str, cast(dict[str, object], item)["raw_sha256"])),
                cls._acceptance_locator(
                    cast(dict[str, object], cast(dict[str, object], item)["locator"])
                ),
            )
            for item in cast(list[object], wire["review_evidence_refs"])
        )
        return ReviewAcceptanceEventInputV3(
            subject_kind=AcceptanceSubjectKindV3(cast(str, wire["subject_kind"])),
            subject_payload_digest_reference=cls._digest_reference_from_wire(
                wire["subject_payload_digest"]
            ),
            reviewer_roster_ref=ReviewerRosterRefV1(
                cast(str, roster_wire["path"]),
                cast(str, roster_wire["schema"]),
                bytes.fromhex(cast(str, roster_wire["raw_sha256"])),
            ),
            reviewer_role_bindings=bindings,
            review_mode=ReviewMode(cast(str, wire["review_mode"])),
            source_binding_digests=sources,
            review_evidence_refs=evidence,
        )

    @staticmethod
    def _acceptance_locator(value: dict[str, object]) -> tuple[str, str | None]:
        kind = cast(str, value["kind"])
        if kind == "whole_artifact":
            return (kind, None)
        return (kind, cast(str, value["value"]))

    def _write_rebound_event(
        self,
        case: dict[str, object],
        record: ContextApplicationV2Record,
        wire: dict[str, object],
    ) -> ContextApplicationV2Record:
        fixture = case["fixture"]
        event_input = self._event_input_from_wire(wire)
        event_wire = ReviewAcceptanceEventLeafV3.from_input(event_input).to_wire()
        raw = (json.dumps(event_wire, separators=(",", ":")) + "\n").encode("utf-8")
        event_id = cast(str, event_wire["event_id"])
        event_path = (
            "sources/m2_5/authorities/review_acceptance_events/v3/"
            + event_id.removeprefix("ae.v3/")
            + ".json"
        )
        fixture.write_repo(event_path, raw)
        event_ref = ReviewEventRefV3(event_path, hashlib.sha256(raw).digest(), event_id)
        return ContextApplicationV2Record.from_parts(
            application_id=record.application_id,
            theorem_record_id=record.theorem_record_id,
            members=record.members,
            review_event_ref_v3=event_ref,
        )

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

    def test_non_record_input_fails_before_filesystem_access(self) -> None:
        from context_application_v2_review_admission import (
            ContextApplicationV2ReviewAdmissionError,
            ContextApplicationV2ReviewAdmissionValidator,
        )

        case = self._synthetic_case()
        validator = ContextApplicationV2ReviewAdmissionValidator(
            cast(object, case["source_resolver"]),
            base_authority_binding=case["base_binding"],
        )
        with self.assertRaises(ContextApplicationV2ReviewAdmissionError) as caught:
            validator.admit(object())  # type: ignore[arg-type]
        self.assertEqual(caught.exception.code, "APPLICATION_INPUT_INVALID")

    def test_role_failure_precedence_is_fixed(self) -> None:
        from context_application_v2_review_admission import (
            ContextApplicationV2ReviewAdmissionError,
            ContextApplicationV2ReviewAdmissionValidator,
        )

        cases = (
            (
                (
                    "rules_authority_maintainer",
                    "conformance_maintainer",
                    "information_safety_reviewer",
                ),
                "architecture_maintainer",
                "REVIEWER_ROLE_MISSING",
            ),
            (
                (
                    "architecture_maintainer",
                    "conformance_maintainer",
                    "information_safety_reviewer",
                ),
                "rules_authority_maintainer",
                "REVIEWER_ROLE_MISSING",
            ),
            (
                (
                    "architecture_maintainer",
                    "rules_authority_maintainer",
                    "information_safety_reviewer",
                ),
                "conformance_maintainer",
                "REVIEWER_ROLE_MISSING",
            ),
            (
                ("architecture_maintainer", "conformance_maintainer", "rules_authority_maintainer"),
                "information_safety_reviewer",
                "INFORMATION_SAFETY_REVIEWER_REQUIRED",
            ),
        )
        for roles, missing_role, expected_code in cases:
            with self.subTest(missing_role=missing_role):
                case = self._synthetic_case()
                source_resolver, record, _ = self._record_with_v3_event(
                    case,
                    reviewer_roles=roles,
                )
                with self.assertRaises(ContextApplicationV2ReviewAdmissionError) as caught:
                    ContextApplicationV2ReviewAdmissionValidator(
                        source_resolver,
                        base_authority_binding=case["base_binding"],
                    ).admit(record)
                self.assertEqual(caught.exception.code, expected_code)
                self.assertEqual(caught.exception.missing_role, missing_role)
                self.assertEqual(
                    caught.exception.location,
                    f"event.reviewer_role_bindings.{missing_role}",
                )

    def test_subject_kind_mismatch_rebinds_record_identity_before_admission(self) -> None:
        from context_application_v2_review_admission import (
            ContextApplicationV2ReviewAdmissionError,
            ContextApplicationV2ReviewAdmissionValidator,
        )

        case = self._synthetic_case()
        source_resolver, record, event_wire = self._record_with_v3_event(case)
        mutated = copy.deepcopy(event_wire)
        mutated["subject_kind"] = "context_application_v2_supersession_record"
        rebound = self._write_rebound_event(case, record, mutated)
        with self.assertRaises(ContextApplicationV2ReviewAdmissionError) as caught:
            ContextApplicationV2ReviewAdmissionValidator(
                source_resolver,
                base_authority_binding=case["base_binding"],
            ).admit(rebound)
        self.assertEqual(caught.exception.code, "V3_SUBJECT_KIND_MISMATCH")

    def test_valid_source_binding_mutations_reach_exact_closure(self) -> None:
        from context_application_v2_review_admission import (
            ContextApplicationV2ReviewAdmissionError,
            ContextApplicationV2ReviewAdmissionValidator,
        )

        case = self._synthetic_case()
        source_resolver, record, event_wire = self._record_with_v3_event(case)
        valid_sources = cast(list[dict[str, object]], event_wire["source_binding_digests"])
        non_roster = next(
            item for item in valid_sources if item["artifact_role"] != "reviewer_roster_leaf"
        )
        mutated = copy.deepcopy(event_wire)
        mutated["source_binding_digests"] = [
            item for item in valid_sources if item is not non_roster
        ]
        missing_record = self._write_rebound_event(case, record, mutated)
        with self.assertRaises(ContextApplicationV2ReviewAdmissionError) as caught:
            ContextApplicationV2ReviewAdmissionValidator(
                source_resolver,
                base_authority_binding=case["base_binding"],
            ).admit(missing_record)
        self.assertEqual(caught.exception.code, "V3_SOURCE_CLOSURE_MISMATCH")

        host_binding = ContextAuthoritySourceBindingV2(
            "host_binding_authority_v2",
            "sources/m2_5/authorities/interaction_review_authority.v2.json",
            "manafold.m2.5.c.interaction-review-authority.v2",
            bytes.fromhex("77" * 32),
        )
        mutated = copy.deepcopy(event_wire)
        mutated["source_binding_digests"] = [
            item
            for item in sorted(
                [*valid_sources, host_binding.to_wire()],
                key=lambda item: encode_canonical(context_source_binding_from_wire(item).to_cbor()),
            )
        ]
        extra_record = self._write_rebound_event(case, record, mutated)
        with self.assertRaises(ContextApplicationV2ReviewAdmissionError) as caught:
            ContextApplicationV2ReviewAdmissionValidator(
                source_resolver,
                base_authority_binding=case["base_binding"],
            ).admit(extra_record)
        self.assertEqual(caught.exception.code, "V3_SOURCE_CLOSURE_MISMATCH")

        mutated = copy.deepcopy(event_wire)
        mutated["source_binding_digests"] = [
            item for item in valid_sources if item["artifact_role"] != "reviewer_roster_leaf"
        ]
        missing_roster_record = self._write_rebound_event(case, record, mutated)
        with self.assertRaises(ContextApplicationV2ReviewAdmissionError) as caught:
            ContextApplicationV2ReviewAdmissionValidator(
                source_resolver,
                base_authority_binding=case["base_binding"],
            ).admit(missing_roster_record)
        self.assertEqual(caught.exception.code, "V3_EVENT_SOURCE_INVALID")

    def test_structural_source_binding_mutations_fail_at_event_source_boundary(self) -> None:
        from context_application_v2_resolver import ContextApplicationV2ResolutionError

        case = self._synthetic_case()
        source_resolver, record, event_wire = self._record_with_v3_event(case)
        repo = cast(Path, case["fixture"].repo)
        event_path = record.review_event_ref_v3.path
        event_file = repo / Path(*event_path.split("/"))
        mutations = (
            lambda sources: sources[0].__setitem__("artifact_role", "unknown"),
            lambda sources: sources[0].__setitem__("schema", "wrong.schema"),
            lambda sources: sources[0].__setitem__("path", "wrong/path"),
            lambda sources: sources.append(copy.deepcopy(sources[0])),
            lambda sources: sources.reverse(),
        )
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                mutated = copy.deepcopy(event_wire)
                sources = cast(list[dict[str, object]], mutated["source_binding_digests"])
                mutation(sources)
                raw = (json.dumps(mutated, separators=(",", ":")) + "\n").encode("utf-8")
                event_file.write_bytes(raw)
                reference = ReviewEventRefV3(
                    event_path,
                    hashlib.sha256(raw).digest(),
                    record.review_event_ref_v3.event_id,
                )
                resolver = ContextApplicationV2Resolver(cast(object, source_resolver))
                with self.assertRaises(ContextApplicationV2ResolutionError) as caught:
                    resolver.resolve_review_event_leaf_v3(reference)
                self.assertEqual(caught.exception.code, "V3_EVENT_SOURCE_INVALID")

    def test_rejected_admission_is_read_only_and_repeatable(self) -> None:
        from context_application_v2_review_admission import (
            ContextApplicationV2ReviewAdmissionError,
            ContextApplicationV2ReviewAdmissionValidator,
        )

        case = self._synthetic_case()
        source_resolver, record, _ = self._record_with_v3_event(
            case,
            reviewer_roles=(
                "architecture_maintainer",
                "conformance_maintainer",
                "rules_authority_maintainer",
            ),
        )
        repo = cast(Path, case["fixture"].repo)

        def file_digests() -> dict[str, str]:
            return {
                path.relative_to(repo).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
                for path in repo.rglob("*")
                if path.is_file()
            }

        before_record = record.to_cbor()
        before_files = file_digests()
        validator = ContextApplicationV2ReviewAdmissionValidator(
            source_resolver,
            base_authority_binding=case["base_binding"],
        )
        errors: list[tuple[str, str, str | None, str | None]] = []
        for _ in range(2):
            with self.assertRaises(ContextApplicationV2ReviewAdmissionError) as caught:
                validator.admit(record)
            errors.append(
                (
                    caught.exception.code,
                    caught.exception.location,
                    caught.exception.cause_code,
                    caught.exception.missing_role,
                )
            )
        self.assertEqual(errors[0], errors[1])
        self.assertEqual(record.to_cbor(), before_record)
        self.assertEqual(file_digests(), before_files)


if __name__ == "__main__":
    unittest.main()
