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
from context_application_v2_test_support import (
    build_application_with_v3_event,
    build_supersession_with_v3_event,
    rebind_application_event,
)
from mtgml.authority import (
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKindV3,
    AcceptanceSubjectPayloadV3,
    ContextApplicationV2Record,
    ContextApplicationV2SupersessionInputV2,
    ContextApplicationV2SupersessionRecord,
    ContextAuthoritySourceBindingV2,
    DigestReferenceV1,
    ReviewAcceptanceEventInputV3,
    ReviewAcceptanceEventLeafV3,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewEventRefV3,
    ReviewMode,
    SupersessionReason,
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
        return build_application_with_v3_event(
            self,
            case,
            review_mode=review_mode,
            reviewer_roles=reviewer_roles,
        )

    def _write_rebound_event(
        self,
        case: dict[str, object],
        record: ContextApplicationV2Record,
        wire: dict[str, object],
    ) -> ContextApplicationV2Record:
        return rebind_application_event(case, record, wire)

    def test_shared_v3_binding_accepts_both_subject_families(self) -> None:
        from context_application_v2_review_binding import admit_v3_review_binding

        case = self._synthetic_case()
        source_resolver, application, _ = build_application_with_v3_event(self, case)
        application_binding = admit_v3_review_binding(
            application,
            source_resolver,
            base_authority_binding=case["base_binding"],
        )
        self.assertEqual(
            application_binding.subject_kind,
            AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD,
        )

        supersession_input = ContextApplicationV2SupersessionInputV2(
            superseded_record_id_bytes=application.record_id.digest_bytes,
            replacement_record_id_bytes=None,
            replacement_record_kind=None,
            reason_code=SupersessionReason.AUTHORITY_REVOCATION,
            source_evidence_refs=(case["member"].member_evidence_refs[0],),
        )
        source_resolver, supersession, _ = build_supersession_with_v3_event(
            self, case, supersession_input
        )
        supersession_binding = admit_v3_review_binding(
            supersession,
            source_resolver,
            base_authority_binding=case["base_binding"],
        )
        self.assertEqual(
            supersession_binding.subject_kind,
            AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_SUPERSESSION_RECORD,
        )
        self.assertEqual(
            supersession_binding.required_roles,
            (
                "architecture_maintainer",
                "rules_authority_maintainer",
                "conformance_maintainer",
                "information_safety_reviewer",
            ),
        )
        self.assertFalse(hasattr(supersession_binding, "event"))
        self.assertFalse(hasattr(supersession_binding, "artifact"))
        self.assertNotIsInstance(supersession_binding, dict)

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

    def test_reviewer_binding_and_subject_digest_negatives_reach_admission(self) -> None:
        from context_application_v2_review_admission import (
            ContextApplicationV2ReviewAdmissionError,
            ContextApplicationV2ReviewAdmissionValidator,
        )

        case = self._synthetic_case()
        source_resolver, record, event_wire = self._record_with_v3_event(case)
        validator = ContextApplicationV2ReviewAdmissionValidator(
            source_resolver,
            base_authority_binding=case["base_binding"],
        )

        unknown_reviewer = copy.deepcopy(event_wire)
        unknown_reviewer["reviewer_role_bindings"][0]["reviewer_id"] = "bob"
        unknown_record = self._write_rebound_event(case, record, unknown_reviewer)
        with self.assertRaises(ContextApplicationV2ReviewAdmissionError) as caught:
            validator.admit(unknown_record)
        self.assertEqual(caught.exception.code, "REVIEWER_BINDING_NOT_IN_ROSTER")

        role_mismatch = copy.deepcopy(event_wire)
        role_mismatch["reviewer_role_bindings"][0]["roles"] = ["architecture_maintainer"]
        role_mismatch_record = self._write_rebound_event(case, record, role_mismatch)
        with self.assertRaises(ContextApplicationV2ReviewAdmissionError) as caught:
            validator.admit(role_mismatch_record)
        self.assertEqual(caught.exception.code, "REVIEWER_BINDING_INVALID")

        duplicate_reviewer = copy.deepcopy(event_wire)
        duplicate_reviewer["reviewer_role_bindings"].append(
            {"reviewer_id": "alice", "roles": ["architecture_maintainer"]}
        )
        duplicate_reviewer["reviewer_role_bindings"] = sorted(
            duplicate_reviewer["reviewer_role_bindings"],
            key=lambda item: encode_canonical([item["reviewer_id"], item["roles"]]),
        )
        duplicate_record = self._write_rebound_event(case, record, duplicate_reviewer)
        with self.assertRaises(ContextApplicationV2ReviewAdmissionError) as caught:
            validator.admit(duplicate_record)
        self.assertEqual(caught.exception.code, "REVIEWER_DUPLICATE")

        supersession_input = ContextApplicationV2SupersessionInputV2(
            superseded_record_id_bytes=record.record_id.digest_bytes,
            replacement_record_id_bytes=None,
            replacement_record_kind=None,
            reason_code=SupersessionReason.AUTHORITY_REVOCATION,
            source_evidence_refs=(case["member"].member_evidence_refs[0],),
        )
        supersession_record = ContextApplicationV2SupersessionRecord.from_parts(
            supersession_id=supersession_input.identity(),
            superseded_record_id=record.record_id,
            replacement_record_id=None,
            reason_code=SupersessionReason.AUTHORITY_REVOCATION,
            source_evidence_refs=supersession_input.source_evidence_refs,
            review_event_ref_v3=record.review_event_ref_v3,
        )
        other_subject = AcceptanceSubjectPayloadV3(
            subject_kind=AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_SUPERSESSION_RECORD,
            subject_payload=supersession_record.acceptance_free_subject_payload(),
        )
        wrong_subject_digest = copy.deepcopy(event_wire)
        wrong_subject_digest["subject_payload_digest"] = DigestReferenceV1.from_identity(
            other_subject.identity()
        ).to_wire()
        wrong_digest_record = self._write_rebound_event(case, record, wrong_subject_digest)
        with self.assertRaises(ContextApplicationV2ReviewAdmissionError) as caught:
            validator.admit(wrong_digest_record)
        self.assertEqual(caught.exception.code, "V3_SUBJECT_DIGEST_MISMATCH")

    def test_review_evidence_missing_and_invalid_are_stable_admission_errors(self) -> None:
        from context_application_v2_review_admission import (
            ContextApplicationV2ReviewAdmissionError,
            ContextApplicationV2ReviewAdmissionValidator,
        )

        case = self._synthetic_case()
        source_resolver, record, event_wire = self._record_with_v3_event(case)
        validator = ContextApplicationV2ReviewAdmissionValidator(
            source_resolver,
            base_authority_binding=case["base_binding"],
        )

        wrong_schema = copy.deepcopy(event_wire)
        wrong_schema["schema"] = "wrong"
        schema_raw = (json.dumps(wrong_schema, separators=(",", ":")) + "\n").encode("utf-8")
        schema_path = record.review_event_ref_v3.path
        schema_file = cast(Path, case["fixture"].repo) / Path(*schema_path.split("/"))
        schema_file.write_bytes(schema_raw)
        wrong_schema_record = ContextApplicationV2Record.from_parts(
            application_id=record.application_id,
            theorem_record_id=record.theorem_record_id,
            members=record.members,
            review_event_ref_v3=ReviewEventRefV3(
                schema_path,
                hashlib.sha256(schema_raw).digest(),
                record.review_event_ref_v3.event_id,
            ),
        )
        with self.assertRaises(ContextApplicationV2ReviewAdmissionError) as caught:
            validator.admit(wrong_schema_record)
        self.assertEqual(caught.exception.code, "V3_EVENT_SCHEMA_INVALID")
        self.assertEqual(caught.exception.cause_code, "SCHEMA_MISMATCH")

        missing_file = copy.deepcopy(event_wire)
        missing_file["review_evidence_refs"][0]["path"] = (
            "docs/review/missing-admission-evidence.md"
        )
        missing_file["review_evidence_refs"][0]["raw_sha256"] = "00" * 32
        missing_file_record = self._write_rebound_event(case, record, missing_file)
        with self.assertRaises(ContextApplicationV2ReviewAdmissionError) as caught:
            validator.admit(missing_file_record)
        self.assertEqual(caught.exception.code, "REVIEW_EVIDENCE_INVALID")
        self.assertEqual(caught.exception.cause_code, "REPOSITORY_SOURCE_MISSING")

        missing_refs = copy.deepcopy(event_wire)
        missing_refs["review_evidence_refs"] = []
        raw = (json.dumps(missing_refs, separators=(",", ":")) + "\n").encode("utf-8")
        event_path = record.review_event_ref_v3.path
        event_file = cast(Path, case["fixture"].repo) / Path(*event_path.split("/"))
        event_file.write_bytes(raw)
        missing_refs_record = ContextApplicationV2Record.from_parts(
            application_id=record.application_id,
            theorem_record_id=record.theorem_record_id,
            members=record.members,
            review_event_ref_v3=ReviewEventRefV3(
                event_path,
                hashlib.sha256(raw).digest(),
                record.review_event_ref_v3.event_id,
            ),
        )
        with self.assertRaises(ContextApplicationV2ReviewAdmissionError) as caught:
            validator.admit(missing_refs_record)
        self.assertEqual(caught.exception.code, "REVIEW_EVIDENCE_MISSING")

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
