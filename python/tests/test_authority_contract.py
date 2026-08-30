from __future__ import annotations

import json
import unittest
from pathlib import Path

import jsonschema
from mtgml.authority import (
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKind,
    AcceptanceSubjectPayloadV1,
    AcceptanceV1,
    AuthorityContractError,
    AuthorityIdentityKind,
    EvidenceRefV1,
    RecordKind,
    ReviewAcceptanceEventInputV1,
    ReviewAcceptanceEventLeafV1,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewerRosterV1,
    ReviewerV1,
    ReviewEventRefV1,
    ReviewMode,
    SourceBindingDigestV1,
    SupersessionReason,
    SupersessionRecordV1,
    canonical_identity_input,
    compute_authority_identity,
)
from mtgml.persistence import decode_canonical

ROOT = Path(__file__).resolve().parents[2]


class AuthorityIdentityTests(unittest.TestCase):
    def test_relation_identity_matches_cross_language_known_answer(self) -> None:
        identity = compute_authority_identity(
            AuthorityIdentityKind.RELATION_THEOREM,
            [
                "manafold.m2.5.c.relation-proof-input.v1",
                "model",
                "positive_interaction",
                "unary",
                "declared_card_trigger",
                "none",
                "not_applicable",
                [[0, "ordered_participant", "card", "subject-ref"]],
                [],
                [
                    "positive_interaction",
                    [
                        [[0, 0, "reads", [], None, None, []]],
                        [],
                        None,
                    ],
                ],
                [],
                [],
            ],
        )

        self.assertEqual(
            identity.as_text(),
            "rp.v1/06dd852fa6a19b5e86d819955ee17cbfaef25d2efa563e1ee67db1368093fddc",
        )
        self.assertEqual(identity.semantic_domain, "manafold.m2.5.c.relation-proof.v1")
        self.assertEqual(
            identity.input_schema_id,
            "manafold.m2.5.c.relation-proof-input.v1",
        )
        self.assertEqual(len(identity.digest_bytes), 32)
        with self.assertRaises(AuthorityContractError):
            canonical_identity_input(
                AuthorityIdentityKind.RELATION_THEOREM,
                ["wrong-schema"],
            )

    def test_foundational_bindings_have_fixed_array_preimages(self) -> None:
        source = SourceBindingDigestV1(
            artifact_role="declared_model",
            path="sources/m2_5/closures/C/declared_interaction_model.v2.json",
            schema_or_null="manafold.m2.5.c.declared-interaction-model.v2",
            raw_sha256=bytes(32),
        )
        reviewer = ReviewerRoleBindingV1(
            reviewer_id="alice",
            roles=("architecture_maintainer", "rules_authority_maintainer"),
        )
        roster_ref = ReviewerRosterRefV1(
            path="sources/m2_5/authorities/reviewer_rosters/v1/" + "00" * 32 + ".json",
            schema="manafold.m2.5.c.reviewer-roster.v1",
            raw_sha256=bytes(32),
        )
        event_ref = ReviewEventRefV1(
            path="sources/m2_5/authorities/review_acceptance_events/v1/" + "00" * 32 + ".json",
            raw_sha256=bytes(32),
            event_id="ae.v1/" + "00" * 32,
        )
        evidence = AcceptanceEvidenceRefV1(
            path="docs/review/authority.md",
            raw_sha256=bytes(32),
            locator=("whole_artifact", None),
        )

        self.assertEqual(
            source.to_cbor(),
            [
                "declared_model",
                "sources/m2_5/closures/C/declared_interaction_model.v2.json",
                "manafold.m2.5.c.declared-interaction-model.v2",
                bytes(32),
            ],
        )
        self.assertEqual(
            reviewer.to_cbor(),
            ["alice", ["architecture_maintainer", "rules_authority_maintainer"]],
        )
        self.assertEqual(
            roster_ref.to_cbor(),
            [
                "sources/m2_5/authorities/reviewer_rosters/v1/" + "00" * 32 + ".json",
                "manafold.m2.5.c.reviewer-roster.v1",
                bytes(32),
            ],
        )
        self.assertEqual(
            event_ref.to_cbor(),
            [
                "sources/m2_5/authorities/review_acceptance_events/v1/" + "00" * 32 + ".json",
                bytes(32),
                ["event_id", "ae.v1/" + "00" * 32],
            ],
        )
        self.assertEqual(
            AcceptanceV1(event_ref).to_cbor(),
            ["human_accepted", event_ref.to_cbor()],
        )
        self.assertEqual(
            evidence.to_cbor(),
            ["docs/review/authority.md", bytes(32), ["whole_artifact", None]],
        )
        with self.assertRaises(AuthorityContractError):
            SourceBindingDigestV1(
                artifact_role="declared_model",
                path="derived/Pair_Interaction_Census_REV3.csv",
                schema_or_null=None,
                raw_sha256=bytes(32),
            )
        with self.assertRaises(AuthorityContractError):
            AcceptanceEvidenceRefV1(
                path="docs/review/authority.md",
                raw_sha256=bytes(32),
                locator=("json_pointer", "/review~2"),
            )

    def test_supersession_rejects_cross_family_replacement(self) -> None:
        source = EvidenceRefV1(
            authority_kind="model",
            path="sources/model.json",
            locator=("whole_artifact", None),
            raw_sha256=bytes(32),
        )
        superseded = compute_authority_identity(
            AuthorityIdentityKind.RELATION_THEOREM_RECORD,
            [
                "manafold.m2.5.c.relation-proof-record-input.v1",
                bytes(32),
                [source.to_cbor()],
                [
                    "sources/m2_5/authorities/review_acceptance_events/v1/" + "00" * 32 + ".json",
                    bytes(32),
                    ["event_id", "ae.v1/" + "00" * 32],
                ],
                "fixture rationale",
            ],
        )
        replacement = compute_authority_identity(
            AuthorityIdentityKind.DOMAIN_THEOREM_RECORD,
            [
                "manafold.m2.5.c.domain-proof-record-input.v1",
                bytes(32),
                [source.to_cbor()],
                [
                    "sources/m2_5/authorities/review_acceptance_events/v1/" + "00" * 32 + ".json",
                    bytes(32),
                    ["event_id", "ae.v1/" + "00" * 32],
                ],
                "fixture rationale",
            ],
        )
        with self.assertRaises(AuthorityContractError):
            SupersessionRecordV1(
                superseded_record_id=superseded,
                replacement_record_id=replacement,
                superseded_record_kind=RecordKind.RELATION_THEOREM_RECORD,
                replacement_record_kind=RecordKind.DOMAIN_THEOREM_RECORD,
                reason_code=SupersessionReason.SEMANTIC_CORRECTION,
                source_evidence_refs=(source,),
                review_event_ref=ReviewEventRefV1(
                    path="sources/m2_5/authorities/review_acceptance_events/v1/"
                    + "00" * 32
                    + ".json",
                    raw_sha256=bytes(32),
                    event_id="ae.v1/" + "00" * 32,
                ),
            )

    def test_acceptance_event_and_roster_have_fixed_identity_inputs(self) -> None:
        subject_evidence = EvidenceRefV1(
            authority_kind="model",
            path="sources/model.json",
            locator=("whole_artifact", None),
            raw_sha256=bytes(32),
        )
        subject = AcceptanceSubjectPayloadV1(
            subject_kind=AcceptanceSubjectKind.RELATION_THEOREM_RECORD,
            subject_payload=[bytes(32), [subject_evidence.to_cbor()], "fixture rationale"],
        )
        roster = ReviewerRosterV1(
            reviewers=(
                ReviewerV1(
                    reviewer_id="alice",
                    roles=("architecture_maintainer", "rules_authority_maintainer"),
                ),
            )
        )
        roster_ref = ReviewerRosterRefV1(
            path="sources/m2_5/authorities/reviewer_rosters/v1/" + "00" * 32 + ".json",
            schema="manafold.m2.5.c.reviewer-roster.v1",
            raw_sha256=bytes(32),
        )
        event = ReviewAcceptanceEventInputV1(
            subject_kind=AcceptanceSubjectKind.RELATION_THEOREM_RECORD,
            subject_payload_digest=bytes(32),
            reviewer_roster_ref=roster_ref,
            reviewer_role_bindings=(
                ReviewerRoleBindingV1(
                    reviewer_id="alice",
                    roles=("architecture_maintainer", "rules_authority_maintainer"),
                ),
            ),
            review_mode=ReviewMode.SOLO_SEPARATE_SELF_REVIEW,
            source_binding_digests=(
                SourceBindingDigestV1(
                    artifact_role="declared_model",
                    path="sources/m2_5/closures/C/declared_interaction_model.v2.json",
                    schema_or_null="manafold.m2.5.c.declared-interaction-model.v2",
                    raw_sha256=bytes(32),
                ),
                SourceBindingDigestV1(
                    artifact_role="reviewer_roster_leaf",
                    path=roster_ref.path,
                    schema_or_null="manafold.m2.5.c.reviewer-roster.v1",
                    raw_sha256=bytes(32),
                ),
            ),
            review_evidence_refs=(
                AcceptanceEvidenceRefV1(
                    path="docs/review/authority.md",
                    raw_sha256=bytes(32),
                    locator=("whole_artifact", None),
                ),
            ),
        )

        self.assertEqual(
            subject.to_cbor(),
            [
                "relation_theorem_record",
                [bytes(32), [subject_evidence.to_cbor()], "fixture rationale"],
            ],
        )
        self.assertTrue(subject.identity().as_text().startswith("asp.v1/"))
        self.assertEqual(roster.to_cbor()[0], "manafold.m2.5.c.reviewer-roster.v1")
        self.assertEqual(
            event.to_cbor()[0],
            "manafold.m2.5.c.review-acceptance-event-input.v1",
        )
        self.assertEqual(
            event.identity().as_text(),
            "ae.v1/605cc0fcb6020f5066896ddc238bc7594e39a7bf731c33a72d88ab7a7acc8013",
        )
        leaf = ReviewAcceptanceEventLeafV1.from_input(event)
        fixture = json.loads(
            (ROOT / "conformance/fixtures/authority/review_acceptance_event.v1.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(leaf.to_wire(), fixture)
        self_binding = SourceBindingDigestV1(
            artifact_role="acceptance_event_leaf",
            path=(
                "sources/m2_5/authorities/review_acceptance_events/v1/"
                + event.identity().as_text().split("/", maxsplit=1)[1]
                + ".json"
            ),
            schema_or_null="manafold.m2.5.c.review-acceptance-event.v1",
            raw_sha256=bytes(32),
        )
        with self.assertRaises(AuthorityContractError):
            ReviewAcceptanceEventInputV1(
                subject_kind=event.subject_kind,
                subject_payload_digest=event.subject_payload_digest,
                reviewer_roster_ref=event.reviewer_roster_ref,
                reviewer_role_bindings=event.reviewer_role_bindings,
                review_mode=event.review_mode,
                source_binding_digests=(self_binding,),
                review_evidence_refs=event.review_evidence_refs,
            )


class AuthoritySchemaTests(unittest.TestCase):
    def test_versioned_authority_schemas_validate_structural_fixtures(self) -> None:
        cases = (
            (
                "schemas/interaction-review-authority.v1.schema.json",
                "conformance/fixtures/authority/interaction_review_authority.v1.json",
            ),
            (
                "schemas/review-acceptance-event.v1.schema.json",
                "conformance/fixtures/authority/review_acceptance_event.v1.json",
            ),
            (
                "schemas/reviewer-roster.v1.schema.json",
                "conformance/fixtures/authority/reviewer_roster.v1.json",
            ),
            (
                "schemas/supersession-record.v1.schema.json",
                "conformance/fixtures/authority/supersession_record.v1.json",
            ),
        )
        for schema_path, fixture_path in cases:
            with self.subTest(schema_path=schema_path):
                schema = json.loads((ROOT / schema_path).read_text(encoding="utf-8"))
                fixture = json.loads((ROOT / fixture_path).read_text(encoding="utf-8"))
                jsonschema.Draft202012Validator(schema).validate(fixture)

    def test_authority_schema_rejects_unknown_top_level_field(self) -> None:
        schema = json.loads(
            (ROOT / "schemas/interaction-review-authority.v1.schema.json").read_text(
                encoding="utf-8"
            )
        )
        fixture = json.loads(
            (
                ROOT / "conformance/fixtures/authority/interaction_review_authority.v1.json"
            ).read_text(encoding="utf-8")
        )
        fixture["unexpected"] = True
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(fixture)

    def test_source_binding_schema_pins_static_role_paths(self) -> None:
        authority_schema = json.loads(
            (ROOT / "schemas/interaction-review-authority.v1.schema.json").read_text(
                encoding="utf-8"
            )
        )
        authority_fixture = json.loads(
            (
                ROOT / "conformance/fixtures/authority/interaction_review_authority.v1.json"
            ).read_text(encoding="utf-8")
        )
        authority_fixture["source_bindings"] = [
            {
                "authority_kind": "model",
                "path": "sources/m2_5/closures/C/declared_interaction_model.v2.json",
                "schema_or_null": "manafold.m2.5.c.declared-interaction-model.v2",
                "raw_sha256": "00" * 32,
                "artifact_role": "declared_model",
            }
        ]
        authority_validator = jsonschema.Draft202012Validator(authority_schema)
        authority_validator.validate(authority_fixture)
        authority_fixture["source_bindings"][0]["path"] = (
            "sources/m2_5/closures/C/not-the-declared-model.json"
        )
        with self.assertRaises(jsonschema.ValidationError):
            authority_validator.validate(authority_fixture)

        event_schema = json.loads(
            (ROOT / "schemas/review-acceptance-event.v1.schema.json").read_text(encoding="utf-8")
        )
        event_fixture = json.loads(
            (ROOT / "conformance/fixtures/authority/review_acceptance_event.v1.json").read_text(
                encoding="utf-8"
            )
        )
        event_validator = jsonschema.Draft202012Validator(event_schema)
        event_validator.validate(event_fixture)
        event_fixture["source_binding_digests"][0]["path"] = (
            "sources/m2_5/closures/C/not-the-declared-model.json"
        )
        with self.assertRaises(jsonschema.ValidationError):
            event_validator.validate(event_fixture)

    def test_nested_authority_schema_closes_projection_and_context_values(self) -> None:
        schema = json.loads(
            (ROOT / "schemas/interaction-review-authority.v1.schema.json").read_text(
                encoding="utf-8"
            )
        )

        def validate_definition(name: str, value: object) -> None:
            probe = {
                "$schema": schema["$schema"],
                "$defs": schema["$defs"],
                "$ref": f"#/$defs/{name}",
            }
            jsonschema.Draft202012Validator(probe).validate(value)

        participant = {
            "position": 0,
            "role": "ordered_participant",
            "participant_kind": "card",
            "semantic_ref": "subject-ref",
        }
        family = {
            "family_id": "cap.fixture",
            "lifecycle": "active",
            "assignment_role": "primary",
        }
        projection = {
            "arity": "unary",
            "directionality": "none",
            "participant_roles": [participant],
            "host_relationship": "not_applicable",
            "context_dimensions": ["not_applicable"] * 10,
            "temporal_semantics": ["not_applicable"] * 4,
            "b2_family_refs": [family],
            "b2_boundary_refs": [],
            "b1_final_citation_refs": [],
        }

        validate_definition("participant_role", participant)
        validate_definition("b2_family_ref", family)
        validate_definition("class_projection", projection)
        validate_definition(
            "semantic_claim_relation",
            {
                "kind": "same_theorem_semantic_id",
                "theorem_semantic_digest": "00" * 32,
            },
        )
        validate_definition(
            "context_slot_attestation",
            {
                "slot_kind": "context_dimension",
                "slot_name": "zone",
                "observed_value": "not_applicable",
                "evidence_refs": [],
                "equivalence_rationale": "fixture",
            },
        )

        invalid_participant = dict(participant, role="not_a_role")
        with self.assertRaises(jsonschema.ValidationError):
            validate_definition("participant_role", invalid_participant)
        invalid_family = dict(family, lifecycle="retired")
        with self.assertRaises(jsonschema.ValidationError):
            validate_definition("b2_family_ref", invalid_family)
        invalid_claim = {"kind": "same_theorem_semantic_id"}
        with self.assertRaises(jsonschema.ValidationError):
            validate_definition("semantic_claim_relation", invalid_claim)
        invalid_slot = {
            "slot_kind": "context_dimension",
            "slot_name": "zone",
            "observed_value": "not_a_zone_value",
            "evidence_refs": [],
            "equivalence_rationale": "fixture",
        }
        with self.assertRaises(jsonschema.ValidationError):
            validate_definition("context_slot_attestation", invalid_slot)


class AuthorityIdentityMatrixTests(unittest.TestCase):
    def test_all_identity_kinds_match_the_shared_golden_matrix(self) -> None:
        matrix = json.loads(
            (ROOT / "conformance/fixtures/authority/identity_golden_matrix.v1.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(len(matrix["identities"]), len(AuthorityIdentityKind))
        equivalence_cases = 0
        context_cases = 0
        for entry in matrix["identities"]:
            with self.subTest(kind=entry["kind"]):
                kind = AuthorityIdentityKind(entry["kind"])
                payload = decode_canonical(bytes.fromhex(entry["payload_cbor_hex"]))
                identity = compute_authority_identity(kind, payload)
                self.assertEqual(identity.prefix, entry["prefix"])
                self.assertEqual(identity.semantic_domain, entry["semantic_domain"])
                self.assertEqual(identity.input_schema_id, entry["input_schema_id"])
                self.assertEqual(len(payload), entry["arity"])
                self.assertEqual(identity.as_text(), entry["identity"])
                if kind is AuthorityIdentityKind.RELATION_APPLICATION:
                    equivalence = payload[3][0][7][1][1]
                    self.assertEqual(equivalence[3][0], "same_theorem_semantic_id")
                    self.assertEqual(len(equivalence[3][1]), 32)
                    equivalence_cases += 1
                if kind is AuthorityIdentityKind.CONTEXT_APPLICATION:
                    self.assertEqual(len(payload[2][0][7][0]), 14)
                    context_cases += 1
        self.assertGreaterEqual(equivalence_cases, 1)
        self.assertGreaterEqual(context_cases, 1)

    def test_identity_producer_rejects_untyped_relation_and_subject_payloads(self) -> None:
        with self.assertRaises(AuthorityContractError):
            compute_authority_identity(
                AuthorityIdentityKind.RELATION_THEOREM,
                [
                    "manafold.m2.5.c.relation-proof-input.v1",
                    "model",
                    "positive_interaction",
                    "unary",
                    "reviewed_relation",
                    "directional",
                    [],
                    [],
                    [],
                    [],
                    [],
                    [],
                ],
            )
        with self.assertRaises(AuthorityContractError):
            compute_authority_identity(
                AuthorityIdentityKind.ACCEPTANCE_SUBJECT,
                [
                    "manafold.m2.5.c.acceptance-subject-payload-input.v1",
                    "relation_theorem_record",
                    [],
                ],
            )

    def test_class_projection_uses_typed_b2_family_refs(self) -> None:
        matrix = json.loads(
            (ROOT / "conformance/fixtures/authority/identity_golden_matrix.v1.json").read_text(
                encoding="utf-8"
            )
        )
        entry = next(item for item in matrix["identities"] if item["kind"] == "relation_theorem")
        payload = decode_canonical(bytes.fromhex(entry["payload_cbor_hex"]))
        payload[9][1][2] = [
            "unary",
            "none",
            [[0, "ordered_participant", "card", "subject-ref"]],
            "not_applicable",
            ["not_applicable"] * 10,
            ["not_applicable"] * 4,
            [["cap.fixture", "active", "primary"]],
            [],
            [],
        ]

        identity = compute_authority_identity(AuthorityIdentityKind.RELATION_THEOREM, payload)
        self.assertTrue(identity.as_text().startswith("rp.v1/"))

    def test_class_projection_equivalence_binds_theorem_digest_and_all_positions(self) -> None:
        matrix = json.loads(
            (ROOT / "conformance/fixtures/authority/identity_golden_matrix.v1.json").read_text(
                encoding="utf-8"
            )
        )
        entry = next(
            item for item in matrix["identities"] if item["kind"] == "relation_application"
        )
        payload = decode_canonical(bytes.fromhex(entry["payload_cbor_hex"]))
        projection = [
            "unary",
            "none",
            [[0, "ordered_participant", "card", "subject-ref"]],
            "not_applicable",
            ["not_applicable"] * 10,
            ["not_applicable"] * 4,
            [],
            [],
            [],
        ]
        payload[3][0][4][4][0][1] = "ordered_participant"
        payload[3][0][4][3] = "not_applicable"
        payload[3][0][7][1][1] = [
            projection,
            projection,
            [
                "arity",
                "directionality",
                "participant_roles",
                "host_relationship",
                "context_dimensions",
                "temporal_semantics",
                "b2_family_refs",
                "b2_boundary_refs",
                "b1_final_citation_refs",
            ],
            ["same_theorem_semantic_id", bytes(32)],
            payload[3][0][6],
            "fixture equivalence",
        ]

        identity = compute_authority_identity(AuthorityIdentityKind.RELATION_APPLICATION, payload)
        self.assertTrue(identity.as_text().startswith("rpa.v1/"))

    def test_context_and_participant_vocabularies_are_closed(self) -> None:
        matrix = json.loads(
            (ROOT / "conformance/fixtures/authority/identity_golden_matrix.v1.json").read_text(
                encoding="utf-8"
            )
        )
        entry = next(item for item in matrix["identities"] if item["kind"] == "context_application")
        payload_bytes = bytes.fromhex(entry["payload_cbor_hex"])
        valid_payload = decode_canonical(payload_bytes)
        identity = compute_authority_identity(
            AuthorityIdentityKind.CONTEXT_APPLICATION, valid_payload
        )
        self.assertTrue(identity.as_text().startswith("cpa.v1/"))

        invalid_payloads = []
        invalid_participant = decode_canonical(payload_bytes)
        invalid_participant[2][0][4][2][0][1] = "not_a_participant_role"
        invalid_payloads.append(invalid_participant)
        invalid_slot = decode_canonical(payload_bytes)
        invalid_slot[2][0][7][0][0][1] = "not_a_context_slot"
        invalid_payloads.append(invalid_slot)
        invalid_value = decode_canonical(payload_bytes)
        invalid_value[2][0][7][0][0][2] = "not_a_zone_value"
        invalid_payloads.append(invalid_value)

        for invalid_payload in invalid_payloads:
            with self.subTest(payload=invalid_payload), self.assertRaises(AuthorityContractError):
                compute_authority_identity(
                    AuthorityIdentityKind.CONTEXT_APPLICATION, invalid_payload
                )

    def test_contract_negative_matrix_rejects_every_case(self) -> None:
        matrix = json.loads(
            (
                ROOT / "conformance/fixtures/authority/identity_contract_negative_matrix.v1.json"
            ).read_text(encoding="utf-8")
        )
        self.assertGreaterEqual(len(matrix["cases"]), 10)
        for case in matrix["cases"]:
            with self.subTest(case_id=case["case_id"]):
                self.assertEqual(case["expected"], "reject")
                payload = decode_canonical(bytes.fromhex(case["payload_cbor_hex"]))
                with self.assertRaises(AuthorityContractError):
                    compute_authority_identity(AuthorityIdentityKind(case["kind"]), payload)

    def test_contract_matrix_positive_controls_are_accepted(self) -> None:
        matrix = json.loads(
            (
                ROOT / "conformance/fixtures/authority/identity_contract_negative_matrix.v1.json"
            ).read_text(encoding="utf-8")
        )
        self.assertGreaterEqual(len(matrix["positive_controls"]), 1)
        for control in matrix["positive_controls"]:
            with self.subTest(control_id=control["control_id"]):
                self.assertEqual(control["expected"], "accept")
                payload = decode_canonical(bytes.fromhex(control["payload_cbor_hex"]))
                compute_authority_identity(AuthorityIdentityKind(control["kind"]), payload)

    def test_required_relation_channels_use_declared_vocabulary_order(self) -> None:
        matrix = json.loads(
            (ROOT / "conformance/fixtures/authority/identity_golden_matrix.v1.json").read_text(
                encoding="utf-8"
            )
        )
        entry = next(item for item in matrix["identities"] if item["kind"] == "relation_theorem")
        payload = decode_canonical(bytes.fromhex(entry["payload_cbor_hex"]))
        payload[9][1][1] = ["participant_boundary", "event_or_effect_causality"]

        identity = compute_authority_identity(AuthorityIdentityKind.RELATION_THEOREM, payload)
        self.assertTrue(identity.as_text().startswith("rp.v1/"))
