from __future__ import annotations

import base64
import hashlib
import json
import sys
import unittest
from dataclasses import FrozenInstanceError, replace
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from authority_source_resolver import (
    AuthoritySourceResolver,
    ResolutionError,
    ResolutionStatus,
    Rev3ArchiveStore,
)
from context_application_v2_host_binding import (
    ContextApplicationV2HostBindingError,
    ContextApplicationV2HostBindingEvaluationResult,
    ContextApplicationV2HostBindingEvaluator,
    _ResolvedApplicationMember,
    _validate_link_member_union,
)
from context_application_v2_test_support import (
    build_application_variant_with_v3_event,
    build_application_with_v3_event,
    build_supersession_with_v3_event,
)
from mtgml.authority import (
    ApplicationHostBindingV2,
    AuthorityContractError,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationAuthorityV2,
    ContextApplicationV2InputV1,
    ContextApplicationV2Record,
    ContextApplicationV2SupersessionInputV2,
    ContextApplicationV2SupersessionRecord,
    ContextAuthoritySourceBindingV2,
    DigestReferenceV1,
    ReviewEventRefV3,
    SupersessionReason,
)
from mtgml.host_binding import ApplicationMemberKeyV1
from mtgml.persistence import encode_canonical
from test_authority_v2_validator import _claim

BASE_BINDING = ContextAuthoritySourceBindingV2(
    "base_authority_v1",
    "sources/m2_5/authorities/interaction_review_authority.v1.json",
    "manafold.m2.5.c.interaction-review-authority.v1",
    bytes([1]) * 32,
)
CANDIDATE_BINDING = ContextAuthoritySourceBindingV2(
    "candidate_universe",
    "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
    "manafold.m2.5.c.interaction-candidate-universe.v2",
    bytes([2]) * 32,
)
HOST_BINDING = ContextAuthoritySourceBindingV2(
    "host_binding_authority_v2",
    "sources/m2_5/authorities/interaction_review_authority.v2.json",
    "manafold.m2.5.c.interaction-review-authority.v2",
    bytes([3]) * 32,
)


def _application_id(marker: int) -> AuthorityIdentityV1:
    return AuthorityIdentityV1(
        AuthorityIdentityKind.CONTEXT_APPLICATION_V2,
        bytes([marker]) * 32,
    )


def _claim_id(marker: int) -> str:
    return "hbc.v1/" + f"{marker:02x}" * 32


def _link(marker: int, claim_marker: int) -> ApplicationHostBindingV2:
    return ApplicationHostBindingV2(
        "context_application",
        _application_id(marker),
        (_claim_id(claim_marker),),
    )


def _empty_container(
    *,
    links: tuple[ApplicationHostBindingV2, ...] = (),
    host_binding: ContextAuthoritySourceBindingV2 | None = None,
) -> ContextApplicationAuthorityV2:
    return ContextApplicationAuthorityV2(
        base_authority_v1_binding=BASE_BINDING,
        host_binding_authority_v2_binding=host_binding,
        candidate_universe_binding=CANDIDATE_BINDING,
        source_bindings=(),
        context_application_v2_records=(),
        context_application_v2_supersession_records=(),
        application_host_bindings_v2=links,
    )


def _file_digests(repo: Path) -> dict[str, str]:
    return {
        path.relative_to(repo).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in repo.rglob("*")
        if path.is_file()
    }


class ContextApplicationV2HostBindingEvaluatorTests(unittest.TestCase):
    def _synthetic_case(self, *, two_candidates: bool = False) -> dict[str, object]:
        from test_context_application_v2_validator import ContextApplicationV2IntegrationTests

        base = ContextApplicationV2IntegrationTests()
        base.setUp()
        self.addCleanup(base.doCleanups)
        self.addCleanup(base.tearDown)
        return base._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
            two_candidates=two_candidates,
        )

    def _mixed_application_case(
        self,
    ) -> tuple[
        dict[str, object],
        AuthoritySourceResolver,
        ContextApplicationV2Record,
        object,
        object,
        ContextAuthoritySourceBindingV2,
    ]:
        from test_authority_source_resolver import (
            REV3_CENSUS_MEMBER,
            REV3_SOURCE_COLUMNS,
            archive_bytes,
            digest,
            json_bytes,
        )

        case = self._synthetic_case(two_candidates=True)
        fixture = case["fixture"]
        universe_path = fixture.repo / Path(
            *["sources", "m2_5", "closures", "C", "interaction_candidate_universe.v2.json"]
        )
        universe = cast(dict[str, object], json.loads(universe_path.read_text(encoding="utf-8")))
        candidates = cast(list[dict[str, object]], universe["candidates"])
        instances = cast(list[dict[str, object]], universe["source_instances"])

        intra_candidate_id = "INTRA_DECK|P2|family.a|family.b|DIRECTIONAL_BINARY"
        intra_source_instance_id = (
            "si.v1/"
            + base64.urlsafe_b64encode(intra_candidate_id.encode("utf-8"))
            .decode("ascii")
            .rstrip("=")
            + "/0"
        )
        rows = [
            fixture._source_values(),
            [
                intra_candidate_id,
                "interaction-model.v1",
                "INTRA_DECK",
                "P2",
                "family.a",
                "family.b",
                "UNORDERED_BINARY",
                "AMBIGUOUS_REQUIRES_REVIEW",
                "candidate classification authority or interaction "
                "trigger is not terminally reviewed",
                '["family.a", "family.b"]',
            ],
        ]
        census_raw = fixture._census_bytes(rows)
        archive_raw = archive_bytes({REV3_CENSUS_MEMBER: census_raw})
        archive = Rev3ArchiveStore.from_bytes(archive_raw, digest(archive_raw))
        census_digest = digest(census_raw)
        archive_digest = digest(archive_raw)

        def source_binding(row_ordinal: int) -> dict[str, object]:
            return {
                "kind": "rev3",
                "archive_member": REV3_CENSUS_MEMBER,
                "archive_member_sha256": census_digest,
                "row_ordinal": row_ordinal,
                "source_columns": list(REV3_SOURCE_COLUMNS),
                "source_values": rows[row_ordinal],
            }

        for index, candidate in enumerate(candidates):
            if index == 1:
                candidate["candidate_id"] = intra_candidate_id
                candidate["scope"] = "intra_deck"
                candidate["relation"] = "unordered_binary"
            candidate["source_binding"] = source_binding(index)
            candidate["candidate_identity"] = fixture._candidate_identity_for_test(candidate)

        instances[0]["source_binding"] = source_binding(0)
        instances[1]["candidate_id"] = intra_candidate_id
        instances[1]["source_instance_id"] = intra_source_instance_id
        instances[1]["source_binding"] = source_binding(1)

        input_bindings = cast(dict[str, object], universe["input_bindings"])
        rev3_input = cast(dict[str, object], input_bindings["rev3_candidate_source"])
        rev3_input["archive_member_sha256"] = census_digest
        rev3_input["source_package_sha256"] = archive_digest
        universe_raw = json_bytes(universe)
        fixture.write_repo(
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            universe_raw,
        )
        candidate_universe_digest = bytes.fromhex(digest(universe_raw))
        source_resolver = AuthoritySourceResolver(fixture.repo, rev3_archive=archive)
        case["source_resolver"] = source_resolver

        base_record = cast(ContextApplicationV2Record, case["record"])
        base_member = base_record.members[0]

        def digest_reference(identity: dict[str, str]) -> DigestReferenceV1:
            return DigestReferenceV1(
                identity["envelope_id"],
                identity["algorithm_id"],
                identity["semantic_domain"],
                identity["payload_codec_id"],
                identity["input_schema_id"],
                bytes.fromhex(identity["digest_hex"]),
            )

        member_a = replace(
            base_member,
            candidate_identity_digest_reference=digest_reference(
                candidates[0]["candidate_identity"]
            ),
            candidate_universe_binding=[
                "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
                "manafold.m2.5.c.interaction-candidate-universe.v2",
                candidate_universe_digest,
            ],
        )
        member_b = replace(
            member_a,
            candidate_id=intra_candidate_id,
            candidate_identity_digest_reference=digest_reference(
                candidates[1]["candidate_identity"]
            ),
            source_instance_id=intra_source_instance_id,
            context_binding_v1=[
                "binary",
                "symmetric",
                member_a.context_binding_v1[2],
                "same_host",
            ],
        )
        members = tuple(
            sorted(
                (member_a, member_b),
                key=lambda member: encode_canonical(
                    [
                        member.candidate_identity_digest_reference.digest_bytes,
                        member.source_instance_id,
                    ]
                ),
            )
        )
        zero_ref = ReviewEventRefV3(
            "sources/m2_5/authorities/review_acceptance_events/v3/" + "00" * 32 + ".json",
            bytes(32),
            "ae.v3/" + "00" * 32,
        )
        mixed_application_id = ContextApplicationV2InputV1(
            theorem_record_id_bytes=base_record.theorem_record_id.digest_bytes,
            members=members,
        ).identity()
        provisional = ContextApplicationV2Record.from_parts(
            application_id=mixed_application_id,
            theorem_record_id=base_record.theorem_record_id,
            members=members,
            review_event_ref_v3=zero_ref,
        )
        candidate_binding = ContextAuthoritySourceBindingV2(
            "candidate_universe",
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            "manafold.m2.5.c.interaction-candidate-universe.v2",
            candidate_universe_digest,
        )
        return case, source_resolver, provisional, member_a, member_b, candidate_binding

    def _evaluator(
        self,
        source_resolver: AuthoritySourceResolver | object | None = None,
    ) -> ContextApplicationV2HostBindingEvaluator:
        if source_resolver is None:
            source_resolver = AuthoritySourceResolver(ROOT)
        return ContextApplicationV2HostBindingEvaluator(source_resolver)

    def test_non_context_authority_input_is_rejected_with_frozen_error(self) -> None:
        evaluator = self._evaluator()

        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            evaluator.evaluate(object())

        error = caught.exception
        self.assertEqual(error.code, "HOST_INTEGRATION_INPUT_INVALID")
        self.assertEqual(error.location, "container")
        self.assertIsNone(error.application_id)
        self.assertEqual(error.subject_ids, ())
        with self.assertRaises(FrozenInstanceError):
            error.code = "changed"

    def test_duplicate_application_host_binding_targets_are_rejected(self) -> None:
        links = tuple(
            sorted(
                (_link(7, 1), _link(7, 2)),
                key=lambda link: encode_canonical(link.to_cbor()),
            )
        )
        container = _empty_container(links=links)

        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            self._evaluator().evaluate(container)

        self.assertEqual(caught.exception.code, "APPLICATION_HOST_BINDING_DUPLICATE")
        self.assertEqual(caught.exception.application_id, _application_id(7))

    def test_unknown_application_host_binding_target_is_rejected(self) -> None:
        container = _empty_container(links=(_link(8, 1),))

        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            self._evaluator().evaluate(container)

        self.assertEqual(
            caught.exception.code,
            "APPLICATION_HOST_BINDING_UNKNOWN_APPLICATION",
        )
        self.assertEqual(caught.exception.application_id, _application_id(8))

    def test_caller_cannot_supply_trusted_currentness_or_claim_inputs(self) -> None:
        evaluator = self._evaluator()
        container = _empty_container()

        with self.assertRaises(TypeError):
            evaluator.evaluate(container, trusted_current=True)
        with self.assertRaises(TypeError):
            evaluator.evaluate(container, admitted_claims_by_id={})

    def test_structural_result_is_frozen_and_currentness_owned(self) -> None:
        result = self._evaluator().evaluate(_empty_container())

        self.assertIsInstance(result, ContextApplicationV2HostBindingEvaluationResult)
        self.assertEqual(result.qualified_current_application_record_ids, ())
        self.assertEqual(result.application_host_binding_results, ())
        self.assertEqual(result.current_host_claim_ids, ())
        with self.assertRaises(FrozenInstanceError):
            result.current_host_claim_ids = ("hbc.v1/" + "00" * 32,)

    def test_member_key_uses_candidate_identity_and_source_instance(self) -> None:
        case = self._synthetic_case()
        record = cast(ContextApplicationV2Record, case["record"])
        evaluator = self._evaluator(case["source_resolver"])

        projections = evaluator._resolve_record_members(
            record,
            cast(ContextAuthoritySourceBindingV2, case["base_binding"]),
        )

        self.assertEqual(len(projections), 1)
        self.assertEqual(
            projections[0].member_key,
            ApplicationMemberKeyV1(
                candidate_id=record.members[0].candidate_id,
                candidate_identity_digest=(
                    record.members[0].candidate_identity_digest_reference.digest_bytes
                ),
                source_instance_id=record.members[0].source_instance_id,
            ),
        )
        self.assertTrue(projections[0].required)

    def test_applicability_uses_only_verified_scope_and_relation(self) -> None:
        case = self._synthetic_case()
        member = cast(ContextApplicationV2Record, case["record"]).members[0]
        evaluator = self._evaluator(case["source_resolver"])

        required = evaluator._project_verified_member(
            member,
            {"scope": "cross_deck", "relation": "directional_binary"},
        )
        non_required = evaluator._project_verified_member(
            member,
            {"scope": "intra_deck", "relation": "unordered_binary"},
        )

        self.assertTrue(required.required)
        self.assertFalse(non_required.required)

    def test_current_and_historical_links_use_the_same_required_subset(self) -> None:
        from context_application_v2_test_support import (
            build_application_with_v3_event,
            build_supersession_with_v3_event,
        )

        case = self._synthetic_case()
        source_resolver, record, _ = build_application_with_v3_event(self, case)
        semantic_input = ContextApplicationV2SupersessionInputV2(
            superseded_record_id_bytes=record.record_id.digest_bytes,
            replacement_record_id_bytes=None,
            replacement_record_kind=None,
            reason_code=SupersessionReason.AUTHORITY_REVOCATION,
            source_evidence_refs=(case["member"].member_evidence_refs[0],),
        )
        _, supersession, _ = build_supersession_with_v3_event(
            self,
            case,
            semantic_input,
        )
        link = ApplicationHostBindingV2(
            "context_application",
            record.application_id,
            (_claim_id(21),),
        )

        current_container = ContextApplicationAuthorityV2(
            base_authority_v1_binding=cast(
                ContextAuthoritySourceBindingV2,
                case["base_binding"],
            ),
            host_binding_authority_v2_binding=HOST_BINDING,
            candidate_universe_binding=CANDIDATE_BINDING,
            source_bindings=(),
            context_application_v2_records=(record,),
            context_application_v2_supersession_records=(),
            application_host_bindings_v2=(link,),
        )
        historical_container = replace(
            current_container,
            context_application_v2_supersession_records=(supersession,),
        )
        evaluator = self._evaluator(source_resolver)

        current = evaluator.evaluate(current_container)
        historical = evaluator.evaluate(historical_container)

        self.assertEqual(current.application_host_binding_results, ())
        self.assertEqual(historical.application_host_binding_results, ())
        self.assertEqual(current.qualified_current_application_record_ids, ())
        self.assertEqual(historical.qualified_current_application_record_ids, ())
        current_closure = evaluator._derive_application_closures(
            (record,),
            current.currentness,
            cast(ContextAuthoritySourceBindingV2, case["base_binding"]),
        )
        historical_closure = evaluator._derive_application_closures(
            (record,),
            historical.currentness,
            cast(ContextAuthoritySourceBindingV2, case["base_binding"]),
        )
        self.assertEqual(
            current_closure[0].required_member_keys,
            historical_closure[0].required_member_keys,
        )

    def test_real_mixed_candidate_records_keep_only_cross_deck_required(self) -> None:
        case, source_resolver, record, member_a, member_b, _candidate_binding = (
            self._mixed_application_case()
        )
        from context_application_v2_supersession import ContextApplicationV2CurrentnessResult

        base_binding = cast(ContextAuthoritySourceBindingV2, case["base_binding"])
        currentness = ContextApplicationV2CurrentnessResult(
            current_record_ids=(record.record_id,),
            superseded_record_ids=(),
            revoked_record_ids=(),
            successor_edges=(),
        )
        historical_currentness = ContextApplicationV2CurrentnessResult(
            current_record_ids=(),
            superseded_record_ids=(record.record_id,),
            revoked_record_ids=(),
            successor_edges=(),
        )
        evaluator = self._evaluator(source_resolver)
        current_closure = evaluator._derive_application_closures(
            (record,),
            currentness,
            base_binding,
        )[0]
        historical_closure = evaluator._derive_application_closures(
            (record,),
            historical_currentness,
            base_binding,
        )[0]
        required_key = ApplicationMemberKeyV1(
            member_a.candidate_id,
            member_a.candidate_identity_digest_reference.digest_bytes,
            member_a.source_instance_id,
        )
        non_required_key = ApplicationMemberKeyV1(
            member_b.candidate_id,
            member_b.candidate_identity_digest_reference.digest_bytes,
            member_b.source_instance_id,
        )
        self.assertEqual(current_closure.required_member_keys, (required_key,))
        self.assertEqual(historical_closure.required_member_keys, (required_key,))
        self.assertNotIn(non_required_key, current_closure.required_member_keys)
        self.assertNotIn(non_required_key, historical_closure.required_member_keys)

    def test_member_source_resolution_errors_are_wrapped_with_typed_boundary(self) -> None:
        case = self._synthetic_case()
        record = cast(ContextApplicationV2Record, case["record"])

        class FailingResolver:
            def resolve_candidate_source_instance(self, *args: object, **kwargs: object) -> object:
                del args, kwargs
                raise ResolutionError(
                    ResolutionStatus.FAIL,
                    "SYNTHETIC_MEMBER_SOURCE_FAILURE",
                    "synthetic source failure",
                )

        evaluator = self._evaluator(FailingResolver())
        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            evaluator._resolve_record_members(
                record,
                cast(ContextAuthoritySourceBindingV2, case["base_binding"]),
            )
        self.assertEqual(caught.exception.code, "HOST_INTEGRATION_INPUT_INVALID")
        self.assertEqual(caught.exception.cause_code, "SYNTHETIC_MEMBER_SOURCE_FAILURE")

    def test_exact_member_union_and_reviewed_host_relationship_are_validated(self) -> None:
        member_a = ApplicationMemberKeyV1("candidate-a", bytes([1]) * 32, "si/a")
        member_b = ApplicationMemberKeyV1("candidate-b", bytes([2]) * 32, "si/b")
        projected = (
            _ResolvedApplicationMember(member_a, "same_host", True),
            _ResolvedApplicationMember(member_b, "same_host", False),
        )
        claim_a = _claim(member_a, "Token Triumph", 1)
        claim_b = _claim(member_b, "Grave Danger", 2)
        claims = {
            claim_a.identity().as_text(): claim_a,
            claim_b.identity().as_text(): claim_b,
        }
        application = _application_id(22)
        mixed_link = ApplicationHostBindingV2(
            "context_application",
            application,
            tuple(sorted(claims)),
        )

        required_members = tuple(member for member in projected if member.required)
        _validate_link_member_union(
            ApplicationHostBindingV2(
                "context_application",
                application,
                (claim_a.identity().as_text(),),
            ),
            required_members,
            claims,
        )

        with self.assertRaises(ContextApplicationV2HostBindingError) as missing:
            _validate_link_member_union(
                mixed_link,
                required_members,
                claims,
            )
        self.assertEqual(missing.exception.code, "HOST_MEMBER_SET_MISMATCH")

        with self.assertRaises(ContextApplicationV2HostBindingError) as relationship:
            _validate_link_member_union(
                mixed_link,
                (
                    _ResolvedApplicationMember(member_a, "same_host", True),
                    _ResolvedApplicationMember(member_b, "cross_host", True),
                ),
                {
                    claim_a.identity().as_text(): claim_a,
                    claim_b.identity().as_text(): claim_b,
                },
            )
        self.assertEqual(relationship.exception.code, "HOST_RELATIONSHIP_MISMATCH")

    def test_noncanonical_link_order_is_rejected_at_constructor_boundary(self) -> None:
        first, second = _link(9, 1), _link(10, 2)
        ordered = tuple(sorted((first, second), key=lambda link: encode_canonical(link.to_cbor())))
        noncanonical = tuple(reversed(ordered))

        with self.assertRaises(AuthorityContractError):
            _empty_container(links=noncanonical)

    def test_forged_typed_noncanonical_container_fails_at_evaluator_boundary(self) -> None:
        valid = _empty_container()
        forged = object.__new__(ContextApplicationAuthorityV2)
        for field_name in (
            "base_authority_v1_binding",
            "host_binding_authority_v2_binding",
            "candidate_universe_binding",
            "source_bindings",
            "context_application_v2_records",
            "context_application_v2_supersession_records",
        ):
            object.__setattr__(forged, field_name, getattr(valid, field_name))
        object.__setattr__(forged, "application_host_bindings_v2", [_link(11, 1)])

        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            self._evaluator().evaluate(forged)

        self.assertEqual(caught.exception.code, "HOST_INTEGRATION_INPUT_INVALID")
        self.assertEqual(caught.exception.location, "application_host_bindings_v2")

    def test_currentness_failure_precedes_host_binding_source_access(self) -> None:
        from test_context_application_v2_validator import ContextApplicationV2IntegrationTests

        base = ContextApplicationV2IntegrationTests()
        base.setUp()
        self.addCleanup(base.doCleanups)
        self.addCleanup(base.tearDown)
        case = base._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
        )
        source_resolver, application_a, _ = build_application_with_v3_event(self, case)
        _, application_b, _ = build_application_variant_with_v3_event(
            self,
            case,
            "cycle-b",
        )

        def supersession(
            source: ContextApplicationV2Record,
            replacement: ContextApplicationV2Record,
        ) -> ContextApplicationV2SupersessionRecord:
            semantic_input = ContextApplicationV2SupersessionInputV2(
                superseded_record_id_bytes=source.record_id.digest_bytes,
                replacement_record_id_bytes=replacement.record_id.digest_bytes,
                replacement_record_kind="context_application_v2_record",
                reason_code=SupersessionReason.SOURCE_REVISION,
                source_evidence_refs=(case["member"].member_evidence_refs[0],),
            )
            _, record, _ = build_supersession_with_v3_event(
                self,
                case,
                semantic_input,
            )
            return record

        a_to_b = supersession(application_a, application_b)
        b_to_a = supersession(application_b, application_a)
        records = tuple(
            sorted(
                (application_a, application_b),
                key=lambda record: encode_canonical(record.to_cbor()),
            )
        )
        supersessions = tuple(
            sorted(
                (a_to_b, b_to_a),
                key=lambda record: encode_canonical(record.to_cbor()),
            )
        )
        container = ContextApplicationAuthorityV2(
            base_authority_v1_binding=cast(
                ContextAuthoritySourceBindingV2,
                case["base_binding"],
            ),
            host_binding_authority_v2_binding=HOST_BINDING,
            candidate_universe_binding=CANDIDATE_BINDING,
            source_bindings=(),
            context_application_v2_records=records,
            context_application_v2_supersession_records=supersessions,
            application_host_bindings_v2=(),
        )
        fixture = case["fixture"]
        before = _file_digests(fixture.repo)

        with self.assertRaises(ContextApplicationV2HostBindingError) as caught:
            self._evaluator(source_resolver).evaluate(container)

        error = caught.exception
        self.assertEqual(error.code, "APPLICATION_CURRENTNESS_FAILED")
        self.assertEqual(error.cause_code, "SUPERSESSION_CYCLE")
        self.assertEqual(_file_digests(fixture.repo), before)


if __name__ == "__main__":
    unittest.main()
