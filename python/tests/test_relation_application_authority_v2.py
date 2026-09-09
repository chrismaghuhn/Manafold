from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from mtgml.authority import (
    AuthorityContractError,
    AuthorityIdentityKind,
    RelationApplicationAuthorityV2,
    RelationAuthoritySourceBindingV2,
    compute_authority_identity,
)
from mtgml.persistence import decode_canonical, encode_canonical
from relation_application_v2_resolver import RelationApplicationV2Resolver
from relation_application_v2_supersession import (
    RelationApplicationV2SupersessionError,
    admit_relation_application_authority_v2,
)


def binding(role: str, path: str, schema: str | None) -> RelationAuthoritySourceBindingV2:
    return RelationAuthoritySourceBindingV2(role, path, schema, b"a" * 32)


class RelationApplicationAuthorityV2Tests(unittest.TestCase):
    def test_rps_and_rpsr_identities_match_shared_golden_bytes(self) -> None:
        matrix = json.loads(
            (
                ROOT
                / "conformance/fixtures/authority/"
                / "relation_application_v2_supersession_identity_golden_matrix.v1.json"
            ).read_text(encoding="utf-8")
        )
        for entry in matrix["identities"]:
            payload = decode_canonical(bytes.fromhex(entry["payload_cbor_hex"]))
            kind = AuthorityIdentityKind(
                "relation_supersession_v2"
                if entry["kind"] == "relation_application_v2_supersession"
                else "relation_supersession_record_v2"
            )
            identity = compute_authority_identity(kind, payload)
            self.assertEqual(identity.as_text(), entry["identity"])
            self.assertEqual(encode_canonical(identity.to_cbor()).hex(), entry["identity_cbor_hex"])

    def test_closure_matrix_is_registered_and_closed(self) -> None:
        matrix = json.loads(
            (
                ROOT
                / "conformance/fixtures/authority/"
                / "relation_application_authority_v2_closure_matrix.v1.json"
            ).read_text(encoding="utf-8")
        )
        self.assertEqual(
            matrix["schema_version"],
            "relation-application-authority-v2-closure-matrix.v1",
        )
        self.assertEqual(
            {case["expected"] for case in matrix["cases"]},
            {"accept", "reject"},
        )

    def _empty_authority(self) -> RelationApplicationAuthorityV2:
        base = binding(
            "base_authority_v1",
            "sources/m2_5/authorities/interaction_review_authority.v1.json",
            "manafold.m2.5.c.interaction-review-authority.v1",
        )
        candidate = binding(
            "candidate_universe",
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            "manafold.m2.5.c.interaction-candidate-universe.v2",
        )
        return RelationApplicationAuthorityV2(
            base,
            candidate,
            tuple(sorted((base, candidate), key=lambda item: item.to_cbor())),
            (),
            (),
        )

    def test_empty_authority_has_valid_zero_current_state(self) -> None:
        authority = self._empty_authority()

        class EmptyResolver:
            def validate_relation_application_authority_v2_source_closure(
                self, value: object
            ) -> tuple[RelationAuthoritySourceBindingV2, ...]:
                return tuple(value.source_bindings)

        result = admit_relation_application_authority_v2(
            authority,
            EmptyResolver(),
            currentness=object(),
        )
        self.assertEqual(result.currentness.current_record_ids, ())

    def test_top_level_projections_must_be_exact_source_entries(self) -> None:
        authority = self._empty_authority()
        wrong_base = binding(
            "base_authority_v1",
            "sources/m2_5/authorities/interaction_review_authority.v1.json",
            "manafold.m2.5.c.interaction-review-authority.v1",
        )
        wrong_base = RelationAuthoritySourceBindingV2(
            wrong_base.artifact_role,
            wrong_base.path,
            wrong_base.schema,
            b"b" * 32,
        )
        with self.assertRaises(AuthorityContractError):
            RelationApplicationAuthorityV2(
                wrong_base,
                authority.candidate_universe_binding,
                authority.source_bindings,
                (),
                (),
            )

    def test_relation_authority_registry_rejects_aggregate_self_binding(self) -> None:
        with self.assertRaises(AuthorityContractError):
            RelationAuthoritySourceBindingV2(
                "relation_application_authority_v2",
                "sources/m2_5/authorities/relation_application_authority/v2/relation_application_authority.v2.json",
                "manafold.m2.5.c.relation-application-authority.v2",
                b"a" * 32,
            )

    def test_container_closure_mismatch_is_fail_closed(self) -> None:
        authority = self._empty_authority()

        class MissingResolver:
            def validate_relation_application_authority_v2_source_closure(
                self, _value: object
            ) -> tuple[RelationAuthoritySourceBindingV2, ...]:
                return ()

        with self.assertRaises(RelationApplicationV2SupersessionError) as raised:
            admit_relation_application_authority_v2(
                authority,
                MissingResolver(),
                currentness=object(),
            )
        self.assertEqual(raised.exception.code, "CONTAINER_SOURCE_CLOSURE_MISMATCH")

    def test_production_resolver_reconstructs_empty_container_closure(self) -> None:
        authority = self._empty_authority()

        class Sources:
            def resolve_repository_artifact(self, *_args: object) -> object:
                return object()

            def resolve_rev3_member(self, *_args: object) -> object:
                return object()

        resolver = RelationApplicationV2Resolver(
            Sources(),
            object(),
            {"schema": "manafold.m2.5.c.interaction-review-authority.v1"},
        )
        closure = resolver.expected_relation_application_authority_v2_source_closure(authority)
        self.assertEqual(closure, authority.source_bindings)


if __name__ == "__main__":
    unittest.main()
