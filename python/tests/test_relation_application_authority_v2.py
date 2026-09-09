from __future__ import annotations

import json
import sys
import unittest
from copy import deepcopy
from pathlib import Path

import jsonschema

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
from test_relation_application_v2_review_admission import valid_record


def binding(role: str, path: str, schema: str | None) -> RelationAuthoritySourceBindingV2:
    return RelationAuthoritySourceBindingV2(role, path, schema, b"a" * 32)


class RelationApplicationAuthorityV2Tests(unittest.TestCase):
    def _schema_and_empty_aggregate(self) -> tuple[dict[str, object], dict[str, object]]:
        schema = json.loads(
            (ROOT / "schemas/relation-application-authority.v2.schema.json").read_text(
                encoding="utf-8"
            )
        )
        aggregate = json.loads(
            (
                ROOT
                / "conformance/fixtures/authority/relation_application_authority.v2.json"
            ).read_text(encoding="utf-8")
        )
        return schema, aggregate

    def test_schema_rejects_rpa_v2_cross_contract_mutations(self) -> None:
        schema, aggregate = self._schema_and_empty_aggregate()
        validator = jsonschema.Draft202012Validator(schema)

        unknown_role = deepcopy(aggregate)
        unknown_role["source_bindings"][0]["artifact_role"] = "unknown_role"
        with self.subTest(case="unknown source role"), self.assertRaises(
            jsonschema.ValidationError
        ):
            validator.validate(unknown_role)

        wrong_path = deepcopy(aggregate)
        wrong_path["source_bindings"][0]["path"] = "sources/not-the-base-authority.json"
        with self.subTest(case="wrong source path"), self.assertRaises(
            jsonschema.ValidationError
        ):
            validator.validate(wrong_path)

        record, _ = valid_record()
        with_record = deepcopy(aggregate)
        with_record["relation_application_v2_records"] = [record.to_wire()]

        wrong_application_kind = deepcopy(with_record)
        wrong_application_kind["relation_application_v2_records"][0]["application_id"] = (
            "rpar.v2/" + "a" * 64
        )
        with self.subTest(case="wrong application identity kind"), self.assertRaises(
            jsonschema.ValidationError
        ):
            validator.validate(wrong_application_kind)

        for field, value in (
            ("scope", "not-a-scope"),
            ("relation", "not-a-relation"),
            ("directionality", "not-a-directionality"),
        ):
            mutated = deepcopy(with_record)
            mutated["relation_application_v2_records"][0]["members"][0]["relation_binding"][
                field
            ] = value
            with self.subTest(case=f"invalid relation {field}"), self.assertRaises(
                jsonschema.ValidationError
            ):
                validator.validate(mutated)

        invalid_role = deepcopy(with_record)
        invalid_role["relation_application_v2_records"][0]["members"][0][
            "relation_binding"
        ]["participant_bindings"][0]["role"] = "not-a-role"
        with self.subTest(case="invalid participant role"), self.assertRaises(
            jsonschema.ValidationError
        ):
            validator.validate(invalid_role)

        invalid_kind = deepcopy(with_record)
        invalid_kind["relation_application_v2_records"][0]["members"][0][
            "relation_binding"
        ]["participant_bindings"][0]["participant_kind"] = "not-a-kind"
        with self.subTest(case="invalid participant kind"), self.assertRaises(
            jsonschema.ValidationError
        ):
            validator.validate(invalid_kind)

        revocation_with_replacement = deepcopy(aggregate)
        revocation_with_replacement["relation_application_v2_supersession_records"] = [
            {
                "record_id": "rpsr.v2/" + "a" * 64,
                "supersession_id": "rps.v2/" + "b" * 64,
                "superseded_record_id": "rpar.v2/" + "c" * 64,
                "replacement_record_id": "rpar.v2/" + "d" * 64,
                "superseded_record_kind": "relation_application_v2_record",
                "replacement_record_kind": "relation_application_v2_record",
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
                        "event_id": "ae.v4/" + "f" * 64,
                        "path": "sources/event.json",
                        "raw_sha256": "f" * 64,
                    },
                },
            }
        ]
        with self.subTest(case="revocation with replacement"), self.assertRaises(
            jsonschema.ValidationError
        ):
            validator.validate(revocation_with_replacement)

        unknown_evidence_authority = deepcopy(with_record)
        unknown_evidence_authority["relation_application_v2_records"][0]["members"][0][
            "member_evidence_refs"
        ][0]["authority_kind"] = "unknown"
        with self.subTest(case="unknown evidence authority kind"), self.assertRaises(
            jsonschema.ValidationError
        ):
            validator.validate(unknown_evidence_authority)

    def test_aggregate_schema_accepts_all_existing_v1_member_proof_wire_goldens(self) -> None:
        schema = json.loads(
            (ROOT / "schemas/relation-application-authority.v2.schema.json").read_text(
                encoding="utf-8"
            )
        )
        aggregate = json.loads(
            (
                ROOT
                / "conformance/fixtures/authority/relation_application_authority.v2.json"
            ).read_text(encoding="utf-8")
        )
        proof_matrix = json.loads(
            (
                ROOT
                / "conformance/fixtures/authority/relation_application_v2_wire_golden.v1.json"
            ).read_text(encoding="utf-8")
        )
        record, _ = valid_record()
        for proof_kind, proof_wire in proof_matrix["proof_wire_goldens"].items():
            with self.subTest(proof_kind=proof_kind):
                candidate = deepcopy(aggregate)
                record_wire = record.to_wire()
                record_wire["members"][0]["member_proof_attestation"] = proof_wire
                candidate["relation_application_v2_records"] = [record_wire]
                jsonschema.Draft202012Validator(schema).validate(candidate)

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
