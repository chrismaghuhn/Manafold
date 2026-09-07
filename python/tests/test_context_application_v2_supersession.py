from __future__ import annotations

import copy
import dataclasses
import hashlib
import json
import sys
import unittest
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from context_application_v2_test_support import (
    build_application_with_v3_event,
    build_supersession_with_v3_event,
    rebind_supersession_event,
)
from mtgml.authority import (
    AcceptanceSubjectKindV3,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationV2Record,
    ContextApplicationV2SupersessionInputV2,
    ContextApplicationV2SupersessionRecord,
    DigestReferenceV1,
    SupersessionReason,
)


class ContextApplicationV2SupersessionAdmissionTests(unittest.TestCase):
    def _synthetic_case(self) -> dict[str, object]:
        from test_context_application_v2_validator import ContextApplicationV2IntegrationTests

        base = ContextApplicationV2IntegrationTests()
        base.setUp()
        self.addCleanup(base.doCleanups)
        self.addCleanup(base.tearDown)
        return base._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
        )

    def _valid_revocation(
        self,
    ) -> tuple[
        dict[str, object],
        object,
        ContextApplicationV2Record,
        ContextApplicationV2SupersessionRecord,
        dict[str, object],
    ]:
        case = self._synthetic_case()
        source_resolver, application, _ = build_application_with_v3_event(self, case)
        semantic_input = ContextApplicationV2SupersessionInputV2(
            superseded_record_id_bytes=application.record_id.digest_bytes,
            replacement_record_id_bytes=None,
            replacement_record_kind=None,
            reason_code=SupersessionReason.AUTHORITY_REVOCATION,
            source_evidence_refs=(case["member"].member_evidence_refs[0],),
        )
        source_resolver, supersession, event_wire = build_supersession_with_v3_event(
            self,
            case,
            semantic_input,
        )
        return case, source_resolver, application, supersession, event_wire

    def test_valid_authority_revocation_is_mechanically_admitted(self) -> None:
        from context_application_v2_supersession import (
            ContextApplicationV2SupersessionAdmissionValidator,
            ContextApplicationV2SupersessionAdmissionResult,
        )

        case, source_resolver, _, supersession, _ = self._valid_revocation()
        result = ContextApplicationV2SupersessionAdmissionValidator(
            source_resolver,
            base_authority_binding=case["base_binding"],
        ).admit(supersession)

        self.assertIsInstance(result, ContextApplicationV2SupersessionAdmissionResult)
        self.assertEqual(result.record_id, supersession.record_id)
        self.assertEqual(result.supersession_id, supersession.supersession_id)
        self.assertIsNone(result.replacement_record_id)
        self.assertEqual(result.reason_code, SupersessionReason.AUTHORITY_REVOCATION)
        self.assertEqual(
            result.subject_digest_reference.semantic_domain,
            "manafold.m2.5.c.acceptance-subject-payload.v3",
        )

    def test_valid_replacement_does_not_require_same_application_id(self) -> None:
        from context_application_v2_supersession import (
            ContextApplicationV2SupersessionAdmissionValidator,
        )

        case = self._synthetic_case()
        source_resolver, application, _ = build_application_with_v3_event(self, case)
        replacement_record_id = AuthorityIdentityV1(
            AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2,
            bytes.fromhex("11" * 32),
        )
        semantic_input = ContextApplicationV2SupersessionInputV2(
            superseded_record_id_bytes=application.record_id.digest_bytes,
            replacement_record_id_bytes=replacement_record_id.digest_bytes,
            replacement_record_kind="context_application_v2_record",
            reason_code=SupersessionReason.SEMANTIC_CORRECTION,
            source_evidence_refs=(case["member"].member_evidence_refs[0],),
        )
        source_resolver, supersession, _ = build_supersession_with_v3_event(
            self,
            case,
            semantic_input,
        )
        result = ContextApplicationV2SupersessionAdmissionValidator(
            source_resolver,
            base_authority_binding=case["base_binding"],
        ).admit(supersession)

        self.assertEqual(result.replacement_record_id, replacement_record_id)
        self.assertEqual(result.reason_code, SupersessionReason.SEMANTIC_CORRECTION)
        self.assertNotEqual(application.application_id, replacement_record_id)

    def test_identity_and_structural_failures_have_stable_categories(self) -> None:
        from context_application_v2_supersession import (
            ContextApplicationV2SupersessionAdmissionValidator,
            ContextApplicationV2SupersessionError,
        )

        case, source_resolver, _, supersession, _ = self._valid_revocation()
        validator = ContextApplicationV2SupersessionAdmissionValidator(
            source_resolver,
            base_authority_binding=case["base_binding"],
        )

        mutations = (
            (
                "SUPERSESSION_IDENTITY_MISMATCH",
                "supersession_id",
                AuthorityIdentityV1(
                    AuthorityIdentityKind.CONTEXT_SUPERSESSION_V2,
                    bytes.fromhex("22" * 32),
                ),
            ),
            (
                "SUPERSESSION_RECORD_IDENTITY_MISMATCH",
                "record_id",
                AuthorityIdentityV1(
                    AuthorityIdentityKind.CONTEXT_SUPERSESSION_RECORD_V2,
                    bytes.fromhex("33" * 32),
                ),
            ),
            (
                "SUPERSESSION_REASON_INVALID",
                "reason_code",
                SupersessionReason.SEMANTIC_CORRECTION,
            ),
        )
        for expected_code, field_name, value in mutations:
            with self.subTest(field_name=field_name):
                original = getattr(supersession, field_name)
                object.__setattr__(supersession, field_name, value)
                try:
                    with self.assertRaises(ContextApplicationV2SupersessionError) as caught:
                        validator.admit(supersession)
                    self.assertEqual(caught.exception.code, expected_code)
                finally:
                    object.__setattr__(supersession, field_name, original)

        wrong_replacement = copy.copy(supersession)
        object.__setattr__(
            wrong_replacement,
            "replacement_record_id",
            AuthorityIdentityV1(
                AuthorityIdentityKind.CONTEXT_APPLICATION_V2,
                bytes.fromhex("44" * 32),
            ),
        )
        with self.assertRaises(ContextApplicationV2SupersessionError) as caught:
            validator.admit(wrong_replacement)
        self.assertEqual(caught.exception.code, "SUPERSESSION_REPLACEMENT_INVALID")

        with self.assertRaises(ContextApplicationV2SupersessionError) as caught:
            validator.admit(object())  # type: ignore[arg-type]
        self.assertEqual(caught.exception.code, "SUPERSESSION_INPUT_INVALID")

    def test_v3_subject_and_closure_failures_are_wrapped(self) -> None:
        from context_application_v2_supersession import (
            ContextApplicationV2SupersessionAdmissionValidator,
            ContextApplicationV2SupersessionError,
        )

        case, source_resolver, _, supersession, event_wire = self._valid_revocation()
        validator = ContextApplicationV2SupersessionAdmissionValidator(
            source_resolver,
            base_authority_binding=case["base_binding"],
        )

        mutations = (
            (
                "wrong_subject_kind",
                lambda wire: wire.__setitem__(
                    "subject_kind",
                    AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD.value,
                ),
                "V3_SUBJECT_KIND_MISMATCH",
            ),
            (
                "wrong_subject_digest",
                lambda wire: cast(dict[str, object], wire["subject_payload_digest"]).__setitem__(
                    "digest_hex",
                    "00" * 32,
                ),
                "V3_SUBJECT_DIGEST_MISMATCH",
            ),
            (
                "missing_source_binding",
                lambda wire: wire.__setitem__(
                    "source_binding_digests",
                    cast(list[object], wire["source_binding_digests"])[1:],
                ),
                "V3_SOURCE_CLOSURE_MISMATCH",
            ),
        )
        for name, mutate, cause_code in mutations:
            with self.subTest(name=name):
                mutated = copy.deepcopy(event_wire)
                mutate(mutated)
                rebound = rebind_supersession_event(case, supersession, mutated)
                with self.assertRaises(ContextApplicationV2SupersessionError) as caught:
                    validator.admit(rebound)
                self.assertEqual(caught.exception.code, "SUPERSESSION_REVIEW_ADMISSION_FAILED")
                self.assertEqual(caught.exception.cause_code, cause_code)

    def test_admission_result_is_frozen_and_rejection_is_read_only(self) -> None:
        from context_application_v2_supersession import (
            ContextApplicationV2SupersessionAdmissionValidator,
            ContextApplicationV2SupersessionError,
        )

        case, source_resolver, _, supersession, _ = self._valid_revocation()
        validator = ContextApplicationV2SupersessionAdmissionValidator(
            source_resolver,
            base_authority_binding=case["base_binding"],
        )
        result = validator.admit(supersession)
        self.assertTrue(dataclasses.is_dataclass(result))
        with self.assertRaises(dataclasses.FrozenInstanceError):
            result.reason_code = SupersessionReason.SOURCE_REVISION  # type: ignore[misc]

        before = supersession.to_cbor()
        with self.assertRaises(ContextApplicationV2SupersessionError):
            invalid = copy.copy(supersession)
            object.__setattr__(
                invalid,
                "record_id",
                AuthorityIdentityV1(
                    AuthorityIdentityKind.CONTEXT_SUPERSESSION_RECORD_V2,
                    bytes.fromhex("55" * 32),
                ),
            )
            validator.admit(invalid)
        self.assertEqual(supersession.to_cbor(), before)


if __name__ == "__main__":
    unittest.main()
