from __future__ import annotations

import copy
import dataclasses
import hashlib
import sys
import unittest
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from context_application_v2_test_support import (
    DEFAULT_REVIEWER_ROLES,
    build_application_record_with_v3_event,
    build_application_variant_with_v3_event,
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
    SupersessionReason,
)
from mtgml.persistence import encode_canonical


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
            ContextApplicationV2SupersessionAdmissionResult,
            ContextApplicationV2SupersessionAdmissionValidator,
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
        _, replacement_application, _ = build_application_variant_with_v3_event(
            self,
            case,
            "replacement-application",
        )
        semantic_input = ContextApplicationV2SupersessionInputV2(
            superseded_record_id_bytes=application.record_id.digest_bytes,
            replacement_record_id_bytes=replacement_application.record_id.digest_bytes,
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

        self.assertEqual(result.replacement_record_id, replacement_application.record_id)
        self.assertEqual(result.reason_code, SupersessionReason.SEMANTIC_CORRECTION)
        self.assertNotEqual(application.application_id, replacement_application.application_id)

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


class ContextApplicationV2CurrentnessGraphTests(unittest.TestCase):
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

    def _application_records(
        self,
    ) -> tuple[
        dict[str, object],
        object,
        ContextApplicationV2Record,
        ContextApplicationV2Record,
        ContextApplicationV2Record,
    ]:
        case = self._synthetic_case()
        source_resolver, application_a, _ = build_application_with_v3_event(self, case)
        _, application_b, _ = build_application_variant_with_v3_event(
            self,
            case,
            "variant-b",
        )
        _, application_c, _ = build_application_variant_with_v3_event(
            self,
            case,
            "variant-c",
        )
        return case, source_resolver, application_a, application_b, application_c

    def _supersession(
        self,
        case: dict[str, object],
        source: ContextApplicationV2Record,
        replacement: ContextApplicationV2Record | None,
        reason: SupersessionReason,
        *,
        reviewer_roles: tuple[str, ...] = DEFAULT_REVIEWER_ROLES,
    ) -> ContextApplicationV2SupersessionRecord:
        semantic_input = ContextApplicationV2SupersessionInputV2(
            superseded_record_id_bytes=source.record_id.digest_bytes,
            replacement_record_id_bytes=(
                None if replacement is None else replacement.record_id.digest_bytes
            ),
            replacement_record_kind=(
                None if replacement is None else "context_application_v2_record"
            ),
            reason_code=reason,
            source_evidence_refs=(case["member"].member_evidence_refs[0],),
        )
        _, record, _ = build_supersession_with_v3_event(
            self,
            case,
            semantic_input,
            reviewer_roles=reviewer_roles,
        )
        return record

    def _evaluate(
        self,
        case: dict[str, object],
        source_resolver: object,
        applications: tuple[ContextApplicationV2Record, ...],
        supersessions: tuple[ContextApplicationV2SupersessionRecord, ...],
    ) -> object:
        from context_application_v2_supersession import (
            ContextApplicationV2CurrentnessEvaluator,
        )

        return ContextApplicationV2CurrentnessEvaluator(
            source_resolver,
            base_authority_binding=case["base_binding"],
        ).evaluate(applications, supersessions)

    @staticmethod
    def _file_digests(case: dict[str, object]) -> dict[str, str]:
        repo = cast(Path, case["fixture"].repo)
        return {
            path.relative_to(repo).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in repo.rglob("*")
            if path.is_file()
        }

    def test_positive_currentness_scenarios(self) -> None:
        from context_application_v2_supersession import (
            ContextApplicationV2CurrentnessResult,
        )

        case, resolver, a, b, c = self._application_records()
        scenarios = (
            ("a-only", (a,), (), (a.record_id,)),
            (
                "a-to-b-cross-application",
                (a, b),
                (self._supersession(case, a, b, SupersessionReason.SOURCE_REVISION),),
                (b.record_id,),
            ),
            (
                "a-to-b-to-c",
                (a, b, c),
                (
                    self._supersession(case, a, b, SupersessionReason.SOURCE_REVISION),
                    self._supersession(case, b, c, SupersessionReason.MODEL_REVISION),
                ),
                (c.record_id,),
            ),
            (
                "a-revoked",
                (a,),
                (
                    self._supersession(
                        case,
                        a,
                        None,
                        SupersessionReason.AUTHORITY_REVOCATION,
                    ),
                ),
                (),
            ),
        )
        for name, applications, supersessions, expected_current in scenarios:
            with self.subTest(name=name):
                result = self._evaluate(case, resolver, applications, supersessions)
                self.assertIsInstance(result, ContextApplicationV2CurrentnessResult)
                if expected_current is not None:
                    self.assertEqual(result.current_record_ids, expected_current)

        b_revoked = self._supersession(
            case,
            b,
            None,
            SupersessionReason.AUTHORITY_REVOCATION,
        )
        result = self._evaluate(
            case,
            resolver,
            (a, b),
            (
                self._supersession(case, a, b, SupersessionReason.SOURCE_REVISION),
                b_revoked,
            ),
        )
        self.assertEqual(result.current_record_ids, ())
        self.assertEqual(result.superseded_record_ids, (a.record_id,))
        self.assertIn(b.record_id, result.revoked_record_ids)

    def _same_application_revision(
        self,
        case: dict[str, object],
        application_record: ContextApplicationV2Record | None = None,
    ) -> ContextApplicationV2Record:
        target = application_record or cast(ContextApplicationV2Record, case["record"])
        _, revision, _ = build_application_record_with_v3_event(
            case,
            target,
            reviewer_roles=(*DEFAULT_REVIEWER_ROLES, "project_owner"),
        )
        return revision

    def test_independent_groups_and_linked_history(self) -> None:
        case, resolver, a, b, c = self._application_records()
        a_revision = self._same_application_revision(case)
        result = self._evaluate(
            case,
            resolver,
            (a, a_revision),
            (self._supersession(case, a, a_revision, SupersessionReason.SOURCE_REVISION),),
        )
        self.assertEqual(result.current_record_ids, (a_revision.record_id,))

        result = self._evaluate(
            case,
            resolver,
            (a, b, c),
            (self._supersession(case, a, b, SupersessionReason.SOURCE_REVISION),),
        )
        self.assertEqual(
            set(result.current_record_ids),
            {b.record_id, c.record_id},
        )

    def test_revocation_scope_is_application_wide_but_not_lineage_wide(self) -> None:
        case, resolver, a, b, _ = self._application_records()
        a_revision = self._same_application_revision(case, a)
        a_to_b = self._supersession(
            case,
            a,
            b,
            SupersessionReason.SOURCE_REVISION,
        )
        x_revocation = self._supersession(
            case,
            a_revision,
            None,
            SupersessionReason.AUTHORITY_REVOCATION,
        )
        result = self._evaluate(
            case,
            resolver,
            (a, a_revision, b),
            (a_to_b, x_revocation),
        )
        self.assertEqual(result.current_record_ids, (b.record_id,))
        self.assertEqual(
            set(result.revoked_record_ids),
            {a.record_id, a_revision.record_id},
        )
        self.assertNotIn(b.record_id, result.revoked_record_ids)
        self.assertTrue(
            any(
                edge.superseded_record_id == a.record_id
                and edge.replacement_record_id == b.record_id
                for edge in result.successor_edges
            )
        )

        b_revision = self._same_application_revision(case, b)
        y_revocation = self._supersession(
            case,
            b_revision,
            None,
            SupersessionReason.AUTHORITY_REVOCATION,
        )
        result = self._evaluate(
            case,
            resolver,
            (a, b, b_revision),
            (a_to_b, y_revocation),
        )
        self.assertEqual(result.current_record_ids, ())
        self.assertNotIn(a.record_id, result.revoked_record_ids)
        self.assertEqual(
            set(result.revoked_record_ids),
            {b.record_id, b_revision.record_id},
        )
        self.assertTrue(
            any(
                edge.superseded_record_id == a.record_id
                and edge.replacement_record_id == b.record_id
                for edge in result.successor_edges
            )
        )

        result = self._evaluate(
            case,
            resolver,
            (a, a_revision),
            (
                self._supersession(
                    case,
                    a,
                    None,
                    SupersessionReason.AUTHORITY_REVOCATION,
                ),
            ),
        )
        self.assertEqual(result.current_record_ids, ())
        self.assertEqual(
            set(result.revoked_record_ids),
            {a.record_id, a_revision.record_id},
        )

    def test_duplicate_cpsr_revisions_materialize_one_edge_with_all_provenance(self) -> None:
        case, resolver, a, b, _ = self._application_records()
        first = self._supersession(case, a, b, SupersessionReason.SOURCE_REVISION)
        second = self._supersession(
            case,
            a,
            b,
            SupersessionReason.SOURCE_REVISION,
            reviewer_roles=(*DEFAULT_REVIEWER_ROLES, "project_owner"),
        )
        result = self._evaluate(case, resolver, (a, b), (first, second))
        self.assertEqual(len(result.successor_edges), 1)
        self.assertEqual(
            result.successor_edges[0].accepted_record_ids,
            tuple(
                sorted(
                    (first.record_id, second.record_id),
                    key=lambda item: encode_canonical(item.to_cbor()),
                )
            ),
        )

    def test_input_permutations_produce_equal_results(self) -> None:
        case, resolver, a, b, c = self._application_records()
        edges = (
            self._supersession(case, a, b, SupersessionReason.SOURCE_REVISION),
            self._supersession(case, b, c, SupersessionReason.MODEL_REVISION),
        )
        first = self._evaluate(case, resolver, (a, b, c), edges)
        second = self._evaluate(case, resolver, (c, a, b), tuple(reversed(edges)))
        self.assertEqual(first, second)

    def test_duplicate_ids_are_rejected_before_admission_and_are_order_independent(
        self,
    ) -> None:
        from context_application_v2_supersession import ContextApplicationV2CurrentnessError

        case, resolver, a, b, _ = self._application_records()
        malformed_application = copy.copy(a)
        object.__setattr__(malformed_application, "members", ())
        fingerprints: list[tuple[object, ...]] = []
        for applications in ((a, malformed_application), (malformed_application, a)):
            with self.assertRaises(ContextApplicationV2CurrentnessError) as caught:
                self._evaluate(case, resolver, applications, ())
            error = caught.exception
            self.assertEqual(error.code, "DUPLICATE_RECORD_ID")
            fingerprints.append(
                (
                    error.code,
                    error.location,
                    error.record_id,
                    error.supersession_id,
                    error.subject_record_ids,
                )
            )
        self.assertEqual(fingerprints[0], fingerprints[1])
        self.assertIsNone(fingerprints[0][3])

        valid_supersession = self._supersession(
            case,
            a,
            b,
            SupersessionReason.SOURCE_REVISION,
        )
        malformed_supersession = copy.copy(valid_supersession)
        object.__setattr__(
            malformed_supersession,
            "reason_code",
            SupersessionReason.AUTHORITY_REVOCATION,
        )
        object.__setattr__(
            malformed_supersession,
            "supersession_id",
            AuthorityIdentityV1(
                AuthorityIdentityKind.CONTEXT_SUPERSESSION_V2,
                bytes.fromhex("88" * 32),
            ),
        )
        fingerprints = []
        for supersessions in (
            (valid_supersession, malformed_supersession),
            (malformed_supersession, valid_supersession),
        ):
            with self.assertRaises(ContextApplicationV2CurrentnessError) as caught:
                self._evaluate(case, resolver, (a, b), supersessions)
            error = caught.exception
            self.assertEqual(error.code, "DUPLICATE_RECORD_ID")
            fingerprints.append(
                (
                    error.code,
                    error.location,
                    error.record_id,
                    error.supersession_id,
                    error.subject_record_ids,
                )
            )
        self.assertEqual(fingerprints[0], fingerprints[1])
        self.assertIsNone(fingerprints[0][3])

        malformed_record_id = copy.copy(a)
        object.__setattr__(malformed_record_id, "record_id", "not-an-identity")
        with self.assertRaises(ContextApplicationV2CurrentnessError) as caught:
            self._evaluate(case, resolver, (malformed_record_id,), ())
        self.assertEqual(caught.exception.code, "CURRENTNESS_INPUT_INVALID")

    def test_distinct_successors_fail_even_when_target_is_equal(self) -> None:
        from context_application_v2_supersession import ContextApplicationV2CurrentnessError

        case, resolver, a, b, _ = self._application_records()
        first = self._supersession(case, a, b, SupersessionReason.SOURCE_REVISION)
        second = self._supersession(case, a, b, SupersessionReason.MODEL_REVISION)
        with self.assertRaises(ContextApplicationV2CurrentnessError) as caught:
            self._evaluate(case, resolver, (a, b), (first, second))
        self.assertEqual(caught.exception.code, "MULTIPLE_SUCCESSORS")
        self.assertEqual(caught.exception.record_id, a.record_id)
        self.assertEqual(
            caught.exception.subject_supersession_ids,
            tuple(
                sorted(
                    (first.supersession_id, second.supersession_id),
                    key=lambda item: encode_canonical(item.to_cbor()),
                )
            ),
        )

    def test_revocation_and_replacement_from_one_source_fail_as_multiple_successors(self) -> None:
        from context_application_v2_supersession import ContextApplicationV2CurrentnessError

        case, resolver, a, b, _ = self._application_records()
        replacement = self._supersession(case, a, b, SupersessionReason.SEMANTIC_CORRECTION)
        revocation = self._supersession(
            case,
            a,
            None,
            SupersessionReason.AUTHORITY_REVOCATION,
        )
        with self.assertRaises(ContextApplicationV2CurrentnessError) as caught:
            self._evaluate(case, resolver, (a, b), (replacement, revocation))
        self.assertEqual(caught.exception.code, "MULTIPLE_SUCCESSORS")

    def test_graph_negative_categories_and_ambiguous_currentness(self) -> None:
        from context_application_v2_supersession import ContextApplicationV2CurrentnessError

        case, resolver, a, b, c = self._application_records()
        unknown = AuthorityIdentityV1(
            AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2,
            bytes.fromhex("66" * 32),
        )
        unknown_input = ContextApplicationV2SupersessionInputV2(
            superseded_record_id_bytes=unknown.digest_bytes,
            replacement_record_id_bytes=b.record_id.digest_bytes,
            replacement_record_kind="context_application_v2_record",
            reason_code=SupersessionReason.SOURCE_REVISION,
            source_evidence_refs=(case["member"].member_evidence_refs[0],),
        )
        _, unknown_source, _ = build_supersession_with_v3_event(
            self,
            case,
            unknown_input,
        )
        with self.assertRaises(ContextApplicationV2CurrentnessError) as caught:
            self._evaluate(case, resolver, (a, b), (unknown_source,))
        self.assertEqual(caught.exception.code, "SUPERSEDED_RECORD_UNKNOWN")

        self_edge = self._supersession(case, a, a, SupersessionReason.SOURCE_REVISION)
        with self.assertRaises(ContextApplicationV2CurrentnessError) as caught:
            self._evaluate(case, resolver, (a,), (self_edge,))
        self.assertEqual(caught.exception.code, "SELF_SUPERSESSION")

        cycle_ab = self._supersession(case, a, b, SupersessionReason.SOURCE_REVISION)
        cycle_ba = self._supersession(case, b, a, SupersessionReason.SOURCE_REVISION)
        with self.assertRaises(ContextApplicationV2CurrentnessError) as caught:
            self._evaluate(case, resolver, (a, b), (cycle_ab, cycle_ba))
        self.assertEqual(caught.exception.code, "SUPERSESSION_CYCLE")

        a_revision = self._same_application_revision(case)
        with self.assertRaises(ContextApplicationV2CurrentnessError) as caught:
            self._evaluate(case, resolver, (a, a_revision), ())
        self.assertEqual(caught.exception.code, "CURRENTNESS_AMBIGUOUS")
        self.assertEqual(caught.exception.application_id, a.application_id)
        self.assertEqual(
            caught.exception.subject_record_ids,
            tuple(
                sorted(
                    (a.record_id, a_revision.record_id),
                    key=lambda item: encode_canonical(item.to_cbor()),
                )
            ),
        )

        with self.assertRaises(ContextApplicationV2CurrentnessError) as caught:
            self._evaluate(case, resolver, (a, a), ())
        self.assertEqual(caught.exception.code, "DUPLICATE_RECORD_ID")

        long_cycle_ab = self._supersession(case, a, b, SupersessionReason.SOURCE_REVISION)
        long_cycle_bc = self._supersession(case, b, c, SupersessionReason.MODEL_REVISION)
        long_cycle_ca = self._supersession(case, c, a, SupersessionReason.SEMANTIC_CORRECTION)
        with self.assertRaises(ContextApplicationV2CurrentnessError) as caught:
            self._evaluate(
                case,
                resolver,
                (a, b, c),
                (long_cycle_ab, long_cycle_bc, long_cycle_ca),
            )
        self.assertEqual(caught.exception.code, "SUPERSESSION_CYCLE")

    def test_supersession_admission_error_is_wrapped_with_structured_cause(self) -> None:
        from context_application_v2_supersession import ContextApplicationV2CurrentnessError

        case, resolver, a, _, _ = self._application_records()
        invalid = self._supersession(
            case,
            a,
            None,
            SupersessionReason.AUTHORITY_REVOCATION,
        )
        object.__setattr__(
            invalid,
            "record_id",
            AuthorityIdentityV1(
                AuthorityIdentityKind.CONTEXT_SUPERSESSION_RECORD_V2,
                bytes.fromhex("77" * 32),
            ),
        )
        with self.assertRaises(ContextApplicationV2CurrentnessError) as caught:
            self._evaluate(case, resolver, (a,), (invalid,))
        self.assertEqual(caught.exception.code, "SUPERSESSION_ADMISSION_FAILED")
        self.assertEqual(caught.exception.cause_code, "SUPERSESSION_RECORD_IDENTITY_MISMATCH")
        self.assertEqual(caught.exception.cause_location, "record_id")
        self.assertEqual(caught.exception.record_id, invalid.record_id)

    def test_graph_error_fingerprint_is_input_order_independent(self) -> None:
        from context_application_v2_supersession import ContextApplicationV2CurrentnessError

        case, resolver, a, b, c = self._application_records()
        first = self._supersession(case, a, b, SupersessionReason.SOURCE_REVISION)
        second = self._supersession(case, a, c, SupersessionReason.MODEL_REVISION)
        errors: list[tuple[object, ...]] = []
        for applications, supersessions in (
            ((a, b, c), (first, second)),
            ((c, a, b), (second, first)),
        ):
            with self.assertRaises(ContextApplicationV2CurrentnessError) as caught:
                self._evaluate(case, resolver, applications, supersessions)
            error = caught.exception
            errors.append(
                (
                    error.code,
                    error.location,
                    error.cause_code,
                    error.cause_location,
                    error.record_id,
                    error.supersession_id,
                    error.application_id,
                    error.subject_record_ids,
                    error.subject_supersession_ids,
                    error.cycle_path,
                )
            )
        self.assertEqual(errors[0], errors[1])

    def test_graph_rejection_preserves_inputs_and_temporary_files(self) -> None:
        from context_application_v2_supersession import ContextApplicationV2CurrentnessError

        case, resolver, a, b, c = self._application_records()
        first = self._supersession(case, a, b, SupersessionReason.SOURCE_REVISION)
        second = self._supersession(case, a, c, SupersessionReason.MODEL_REVISION)
        applications_before = tuple(record.to_cbor() for record in (a, b, c))
        supersessions_before = tuple(record.to_cbor() for record in (first, second))
        files_before = self._file_digests(case)

        for applications, supersessions in (
            ((a, b, c), (first, second)),
            ((c, b, a), (second, first)),
        ):
            with self.assertRaises(ContextApplicationV2CurrentnessError):
                self._evaluate(case, resolver, applications, supersessions)
            self.assertEqual(
                tuple(record.to_cbor() for record in (a, b, c)),
                applications_before,
            )
            self.assertEqual(
                tuple(record.to_cbor() for record in (first, second)),
                supersessions_before,
            )
            self.assertEqual(self._file_digests(case), files_before)


if __name__ == "__main__":
    unittest.main()
