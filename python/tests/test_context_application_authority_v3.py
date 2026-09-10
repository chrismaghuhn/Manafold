from __future__ import annotations

import copy
import json
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace

import jsonschema

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from context_application_v3_resolver import (
    ContextApplicationV3AuthorityResolver,
    ContextApplicationV3ResolutionError,
)
from mtgml.authority import (
    ApplicationHostBindingV3,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationAuthorityV3,
    ContextAuthoritySourceBindingV3,
    RelationAuthoritySourceBindingV2,
)
from mtgml.host_binding import HostBindingSourceBindingV2
from mtgml.persistence import encode_canonical


def binding(role: str, digest: bytes) -> ContextAuthoritySourceBindingV3:
    values = {
        "base_authority_v1": (
            "sources/m2_5/authorities/interaction_review_authority.v1.json",
            "manafold.m2.5.c.interaction-review-authority.v1",
        ),
        "candidate_universe": (
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            "manafold.m2.5.c.interaction-candidate-universe.v2",
        ),
        "relation_authority_v2": (
            "sources/m2_5/authorities/relation_application_authority/v2/relation_application_authority.v2.json",
            "manafold.m2.5.c.relation-application-authority.v2",
        ),
    }
    path, schema = values[role]
    return ContextAuthoritySourceBindingV3(role, path, schema, digest)


class ContextApplicationAuthorityV3Tests(unittest.TestCase):
    def test_total_context_closure_includes_relation_and_host_provenance(self) -> None:
        base = binding("base_authority_v1", b"a" * 32)
        candidate = binding("candidate_universe", b"b" * 32)
        relation_projection = binding("relation_authority_v2", b"c" * 32)
        model = ContextAuthoritySourceBindingV3(
            "declared_model",
            "sources/m2_5/closures/C/declared_interaction_model.v2.json",
            "manafold.m2.5.c.declared-interaction-model.v2",
            b"m" * 32,
        )
        host_authority = ContextAuthoritySourceBindingV3(
            "host_binding_authority_v2",
            "sources/m2_5/authorities/interaction_review_authority.v2.json",
            "manafold.m2.5.c.interaction-review-authority.v2",
            b"h" * 32,
        )
        claim_path = (
            "sources/m2_5/authorities/cross_deck_host_binding_claims/v1/" + "d" * 64 + ".json"
        )
        claim_context = ContextAuthoritySourceBindingV3(
            "host_binding_claim_record",
            claim_path,
            "manafold.m2.5.c.cross-deck-host-binding-claim-record.v1",
            b"d" * 32,
        )
        relation_sources = tuple(
            sorted(
                (
                    RelationAuthoritySourceBindingV2(
                        "base_authority_v1", base.path, base.schema, base.raw_sha256
                    ),
                    RelationAuthoritySourceBindingV2(
                        "candidate_universe", candidate.path, candidate.schema, candidate.raw_sha256
                    ),
                    RelationAuthoritySourceBindingV2(
                        "declared_model", model.path, model.schema, model.raw_sha256
                    ),
                ),
                key=lambda item: encode_canonical(item.to_cbor()),
            )
        )
        host_sources = tuple(
            sorted(
                (
                    HostBindingSourceBindingV2(
                        "base_authority_v1", base.path, base.schema, base.raw_sha256
                    ),
                    HostBindingSourceBindingV2(
                        "candidate_universe", candidate.path, candidate.schema, candidate.raw_sha256
                    ),
                    HostBindingSourceBindingV2(
                        "host_binding_claim_record",
                        claim_path,
                        claim_context.schema,
                        claim_context.raw_sha256,
                    ),
                ),
                key=lambda item: encode_canonical(item.to_cbor()),
            )
        )
        source_bindings = tuple(
            sorted(
                (base, candidate, relation_projection, model, host_authority, claim_context),
                key=lambda item: encode_canonical(item.to_cbor()),
            )
        )
        authority = ContextApplicationAuthorityV3(
            base,
            candidate,
            source_bindings,
            relation_projection,
            relation_sources,
            host_authority,
            host_sources,
            (),
            (),
            (
                ApplicationHostBindingV3(
                    "context_application_v3",
                    AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_APPLICATION_V3, b"q" * 32),
                    ("hbc.v1/" + "e" * 64,),
                ),
            ),
        )

        class SourceResolver:
            def resolve_repository_artifact(self, *_args: object) -> object:
                return SimpleNamespace(json_value={"bound": "rpa"})

        class RelationResolver:
            def validate_relation_application_authority_v2_source_closure(
                self, _authority: object
            ) -> tuple[RelationAuthoritySourceBindingV2, ...]:
                return relation_sources

        context_resolver = SimpleNamespace(
            _source_resolver=SourceResolver(),
            _rpa_member_resolver=SimpleNamespace(_rpa_resolver=RelationResolver()),
            resolve_acceptance_event_leaf_v4=lambda _reference: None,
        )
        relation_authority = SimpleNamespace(
            base_authority_v1_binding=next(
                item for item in relation_sources if item.artifact_role == "base_authority_v1"
            ),
            candidate_universe_binding=next(
                item for item in relation_sources if item.artifact_role == "candidate_universe"
            ),
            source_bindings=relation_sources,
            to_wire=lambda: {"bound": "rpa"},
        )
        context_resolver._rpa_member_resolver._authority = relation_authority
        evaluator = ContextApplicationV3AuthorityResolver(context_resolver)
        evaluator.validate_source_closure(
            authority,
            relation_authority=relation_authority,
            host_read_model=SimpleNamespace(used_source_bindings=host_sources),
        )

        missing_claim = ContextApplicationAuthorityV3(
            base,
            candidate,
            tuple(
                sorted(
                    (base, candidate, relation_projection, model, host_authority),
                    key=lambda item: encode_canonical(item.to_cbor()),
                )
            ),
            relation_projection,
            relation_sources,
            host_authority,
            host_sources,
            (),
            (),
            authority.application_host_bindings_v3,
        )
        with self.assertRaises(ContextApplicationV3ResolutionError):
            evaluator.validate_source_closure(
                missing_claim,
                relation_authority=relation_authority,
                host_read_model=SimpleNamespace(used_source_bindings=host_sources),
            )

    def test_v3_schema_pins_source_roles_and_paths(self) -> None:
        schema = json.loads(
            (ROOT / "schemas/context-application-authority.v3.schema.json").read_text(
                encoding="utf-8"
            )
        )
        fixture = json.loads(
            (
                ROOT / "conformance/fixtures/authority/context_application_authority.v3.json"
            ).read_text(encoding="utf-8")
        )
        validator = jsonschema.Draft202012Validator(schema)
        validator.validate(fixture)
        invalid = copy.deepcopy(fixture)
        invalid["source_bindings"][0]["path"] = "wrong.json"
        with self.assertRaises(jsonschema.ValidationError):
            validator.validate(invalid)

        invalid_supersession = copy.deepcopy(fixture)
        invalid_supersession["context_application_v3_supersession_records"] = [
            {
                "record_id": "cpsr.v3/" + "0" * 64,
                "supersession_id": "cps.v3/" + "1" * 64,
                "superseded_record_id": "cpar.v3/" + "2" * 64,
                "replacement_record_id": "cpar.v3/" + "3" * 64,
                "superseded_record_kind": "context_application_v3_record",
                "replacement_record_kind": "context_application_v3_record",
                "reason_code": "authority_revocation",
                "source_evidence_refs": [
                    {
                        "authority_kind": "model",
                        "path": "sources/model.json",
                        "locator": {"kind": "whole_artifact"},
                        "raw_sha256": "e" * 64,
                    }
                ],
                "acceptance": {
                    "decision": "human_accepted",
                    "review_event_ref": {
                        "event_id": "ae.v4/" + "4" * 64,
                        "path": "sources/m2_5/authorities/review_acceptance_events/v4/"
                        + "4" * 64
                        + ".json",
                        "raw_sha256": "4" * 64,
                    },
                },
            }
        ]
        with self.assertRaises(jsonschema.ValidationError):
            validator.validate(invalid_supersession)

    def test_relation_source_bindings_reject_duplicate_role_path(self) -> None:
        base = binding("base_authority_v1", b"a" * 32)
        candidate = binding("candidate_universe", b"b" * 32)
        relation = binding("relation_authority_v2", b"c" * 32)
        relation_binding = RelationAuthoritySourceBindingV2(
            "declared_model",
            "sources/m2_5/closures/C/declared_interaction_model.v2.json",
            "manafold.m2.5.c.declared-interaction-model.v2",
            b"m" * 32,
        )
        conflicting = RelationAuthoritySourceBindingV2(
            "declared_model",
            "sources/m2_5/closures/C/declared_interaction_model.v2.json",
            "manafold.m2.5.c.declared-interaction-model.v2",
            b"z" * 32,
        )
        with self.assertRaises(ValueError):
            ContextApplicationAuthorityV3(
                base,
                candidate,
                tuple(sorted((base, candidate, relation), key=lambda item: item.to_cbor())),
                relation,
                (relation_binding, conflicting),
                None,
                (),
                (),
                (),
                (),
            )

    def test_cross_version_eligibility_rejects_duplicate_current_member(self) -> None:
        key = (b"c" * 32, "si/0")
        with self.assertRaises(ContextApplicationV3ResolutionError) as raised:
            ContextApplicationV3AuthorityResolver.require_cross_version_eligibility(
                v2_member_facts=((key, ("ordered_participant",), ("ordered_participant",)),),
                v3_member_facts=((key, ("ordered_participant",), ("source",)),),
            )
        self.assertEqual(
            raised.exception.code,
            "CONTEXT_AUTHORITY_VERSION_AMBIGUOUS",
        )

    def test_cross_version_eligibility_rejects_divergent_member_on_v2_path(self) -> None:
        key = (b"c" * 32, "si/0")
        with self.assertRaises(ContextApplicationV3ResolutionError) as raised:
            ContextApplicationV3AuthorityResolver.require_cross_version_eligibility(
                v2_member_facts=((key, ("ordered_participant",), ("source",)),),
                v3_member_facts=(),
            )
        self.assertEqual(
            raised.exception.code,
            "CONTEXT_AUTHORITY_VERSION_ELIGIBILITY_MISMATCH",
        )

    def test_currentness_requires_secure_record_and_supersession_admission(self) -> None:
        base = binding("base_authority_v1", b"a" * 32)
        candidate = binding("candidate_universe", b"b" * 32)
        relation = binding("relation_authority_v2", b"c" * 32)
        authority = ContextApplicationAuthorityV3(
            base,
            candidate,
            tuple(sorted((base, candidate, relation), key=lambda item: item.to_cbor())),
            relation,
            (),
            None,
            (),
            (),
            (),
            (),
        )
        with self.assertRaises(ContextApplicationV3ResolutionError) as raised:
            ContextApplicationV3AuthorityResolver().evaluate_currentness(authority)
        self.assertEqual(
            raised.exception.code,
            "CONTEXT_APPLICATION_V3_CURRENTNESS_ADMISSION_REQUIRED",
        )

    def test_empty_authority_has_exact_projection_shape(self) -> None:
        base = binding("base_authority_v1", b"a" * 32)
        candidate = binding("candidate_universe", b"b" * 32)
        relation = binding("relation_authority_v2", b"c" * 32)
        source_bindings = tuple(
            sorted((base, candidate, relation), key=lambda item: item.to_cbor())
        )
        authority = ContextApplicationAuthorityV3(
            base,
            candidate,
            source_bindings,
            relation,
            (),
            None,
            (),
            (),
            (),
            (),
        )
        wire = authority.to_wire()
        self.assertEqual(wire["schema"], "manafold.m2.5.c.context-application-authority.v3")
        self.assertEqual(wire["context_application_v3_records"], [])

    def test_source_closure_requires_reconstructed_relation_closure(self) -> None:
        base = binding("base_authority_v1", b"a" * 32)
        candidate = binding("candidate_universe", b"b" * 32)
        relation = binding("relation_authority_v2", b"c" * 32)
        authority = ContextApplicationAuthorityV3(
            base,
            candidate,
            tuple(sorted((base, candidate, relation), key=lambda item: item.to_cbor())),
            relation,
            (),
            None,
            (),
            (),
            (),
            (),
        )
        with self.assertRaises(ContextApplicationV3ResolutionError) as raised:
            ContextApplicationV3AuthorityResolver().validate_source_closure(
                authority,
                relation_authority=SimpleNamespace(source_bindings=()),
            )
        self.assertEqual(
            raised.exception.code,
            "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH",
        )

    def test_relation_authority_object_must_match_bound_artifact(self) -> None:
        base = binding("base_authority_v1", b"a" * 32)
        candidate = binding("candidate_universe", b"b" * 32)
        relation = binding("relation_authority_v2", b"c" * 32)
        authority = ContextApplicationAuthorityV3(
            base,
            candidate,
            tuple(sorted((base, candidate, relation), key=lambda item: item.to_cbor())),
            relation,
            (),
            None,
            (),
            (),
            (),
            (),
        )

        class SourceResolver:
            def resolve_repository_artifact(self, *_args: object) -> object:
                return SimpleNamespace(json_value={"schema": "bound"})

        resolver = object.__new__(ContextApplicationV3AuthorityResolver)
        resolver._resolver = SimpleNamespace(_source_resolver=SourceResolver())
        with self.assertRaises(ContextApplicationV3ResolutionError) as raised:
            resolver._require_bound_relation_authority(
                authority,
                SimpleNamespace(to_wire=lambda: {"schema": "substituted"}),
            )
        self.assertEqual(
            raised.exception.code,
            "CONTEXT_APPLICATION_V3_RELATION_AUTHORITY_MISMATCH",
        )

    def test_rpa_member_resolver_must_use_the_bound_authority_object(self) -> None:
        resolver = object.__new__(ContextApplicationV3AuthorityResolver)
        resolver._resolver = SimpleNamespace(
            _rpa_member_resolver=SimpleNamespace(
                _authority=SimpleNamespace(to_wire=lambda: {"authority": "A"})
            )
        )
        with self.assertRaises(ContextApplicationV3ResolutionError) as raised:
            resolver._require_rpa_member_resolver_binding(
                SimpleNamespace(to_wire=lambda: {"authority": "B"})
            )
        self.assertEqual(
            raised.exception.code,
            "CONTEXT_APPLICATION_V3_RELATION_AUTHORITY_MISMATCH",
        )

    def test_closure_matrix_cases_are_executable(self) -> None:
        matrix = json.loads(
            (
                ROOT
                / "conformance/fixtures/authority/context_application_v3_closure_matrix.v1.json"
            ).read_text(encoding="utf-8")
        )
        base = binding("base_authority_v1", b"a" * 32)
        candidate = binding("candidate_universe", b"b" * 32)
        relation = binding("relation_authority_v2", b"c" * 32)

        def raw_authority(*, links: tuple[object, ...], host_sources: tuple[object, ...]) -> object:
            value = object.__new__(ContextApplicationAuthorityV3)
            object.__setattr__(value, "base_authority_v1_binding", base)
            object.__setattr__(value, "candidate_universe_binding", candidate)
            object.__setattr__(value, "source_bindings", (base, candidate, relation))
            object.__setattr__(value, "relation_application_authority_v2_binding", relation)
            object.__setattr__(value, "relation_source_bindings", ())
            object.__setattr__(value, "host_binding_authority_v2_binding", None)
            object.__setattr__(value, "host_binding_source_bindings", host_sources)
            object.__setattr__(value, "context_application_v3_records", ())
            object.__setattr__(value, "context_application_v3_supersession_records", ())
            object.__setattr__(value, "application_host_bindings_v3", links)
            return value

        for case in matrix["cases"]:
            with self.subTest(case=case["name"]):
                if case["name"] == "empty_authority_no_host_binding":
                    ContextApplicationV3AuthorityResolver.validate_container_shape(
                        raw_authority(links=(), host_sources=())
                    )
                elif case["name"] == "shared_snapshot_mismatch":
                    with self.assertRaises(ContextApplicationV3ResolutionError) as raised:
                        ContextApplicationV3AuthorityResolver.require_shared_snapshots(
                            SimpleNamespace(
                                base_authority_v1_binding=base,
                                candidate_universe_binding=candidate,
                            ),
                            SimpleNamespace(
                                base_authority_v1_binding=base,
                                candidate_universe_binding=binding("candidate_universe", b"z" * 32),
                            ),
                        )
                    self.assertEqual(raised.exception.code, case["expected_error"])
                elif case["name"] == "host_link_without_authority":
                    with self.assertRaises(ContextApplicationV3ResolutionError) as raised:
                        ContextApplicationV3AuthorityResolver.validate_container_shape(
                            raw_authority(links=(object(),), host_sources=())
                        )
                    self.assertEqual(raised.exception.code, case["expected_error"])
                elif case["name"] == "host_sources_without_link":
                    with self.assertRaises(ContextApplicationV3ResolutionError) as raised:
                        ContextApplicationV3AuthorityResolver.validate_container_shape(
                            raw_authority(links=(), host_sources=(object(),))
                        )
                    self.assertEqual(raised.exception.code, case["expected_error"])

    def test_shared_snapshot_mismatch_fails_before_composition(self) -> None:
        base = binding("base_authority_v1", b"a" * 32)
        candidate = binding("candidate_universe", b"b" * 32)
        authority = SimpleNamespace(
            base_authority_v1_binding=base,
            candidate_universe_binding=candidate,
        )
        relation_authority = SimpleNamespace(
            base_authority_v1_binding=base,
            candidate_universe_binding=binding("candidate_universe", b"z" * 32),
        )
        with self.assertRaises(ContextApplicationV3ResolutionError) as raised:
            ContextApplicationV3AuthorityResolver.require_shared_snapshots(
                authority, relation_authority
            )
        self.assertEqual(raised.exception.code, "CONTEXT_APPLICATION_V3_SHARED_SNAPSHOT_MISMATCH")

    def test_shared_snapshot_mismatch_covers_declared_model_sources(self) -> None:
        base = binding("base_authority_v1", b"a" * 32)
        candidate = binding("candidate_universe", b"b" * 32)
        model = ContextAuthoritySourceBindingV3(
            "declared_model",
            "sources/m2_5/closures/C/declared_interaction_model.v2.json",
            "manafold.m2.5.c.declared-interaction-model.v2",
            b"m" * 32,
        )
        context_authority = SimpleNamespace(
            base_authority_v1_binding=base,
            candidate_universe_binding=candidate,
            source_bindings=(base, candidate, model),
        )
        relation_authority = SimpleNamespace(
            base_authority_v1_binding=base,
            candidate_universe_binding=candidate,
            source_bindings=(
                base,
                candidate,
                ContextAuthoritySourceBindingV3(
                    "declared_model",
                    "sources/m2_5/closures/C/declared_interaction_model.v2.json",
                    "manafold.m2.5.c.declared-interaction-model.v2",
                    b"z" * 32,
                ),
            ),
        )
        with self.assertRaises(ContextApplicationV3ResolutionError) as raised:
            ContextApplicationV3AuthorityResolver.require_shared_snapshots(
                context_authority, relation_authority
            )
        self.assertEqual(raised.exception.code, "CONTEXT_APPLICATION_V3_SHARED_SNAPSHOT_MISMATCH")

    def test_v3_host_link_is_typed_to_cpa_semantic_identity(self) -> None:
        identity = AuthorityIdentityV1(AuthorityIdentityKind.CONTEXT_APPLICATION_V3, b"d" * 32)
        link = ApplicationHostBindingV3(
            "context_application_v3",
            identity,
            ("hbc.v1/" + "e" * 64,),
        )
        self.assertEqual(link.to_cbor()[1], b"d" * 32)
        self.assertNotEqual(link.to_cbor()[1], identity.as_text())


if __name__ == "__main__":
    unittest.main()
