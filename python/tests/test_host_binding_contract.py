from __future__ import annotations

import json
import unittest
from pathlib import Path

import jsonschema
from mtgml.authority import (
    AcceptanceEvidenceRefV1,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewMode,
)
from mtgml.host_binding import (
    HOST_BINDING_AUTHORITY_SCHEMA_V2,
    ApplicationHostBindingV1,
    ApplicationMemberKeyV1,
    CrossDeckHostBindingClaimRecordV1,
    CrossDeckHostBindingClaimSupersessionV1,
    CrossDeckHostBindingClaimV1,
    CrossDeckParticipantDiscoveryHostBindingV1,
    DiscoveryHostRefV1,
    HostBindingAcceptanceEventInputV2,
    HostBindingAcceptanceEventLeafV2,
    HostBindingAcceptanceEventRefV2,
    HostBindingContractError,
    HostBindingEvidenceRefV2,
    HostBindingSourceBindingV2,
    HostRealizationWitnessV1,
    ParticipantHostRealizationV1,
)

ROOT = Path(__file__).resolve().parents[2]


def _ref(role: str, path: str, row: int) -> HostBindingEvidenceRefV2:
    schema = (
        None if role.startswith("rev3_") else "manafold.m2.5.b2.card-semantic-classifications.v1"
    )
    return HostBindingEvidenceRefV2(
        artifact_role=role,
        path=path,
        schema_or_null=schema,
        raw_sha256=bytes(32),
        locator=("csv_row", row),
    )


def _member() -> ApplicationMemberKeyV1:
    return ApplicationMemberKeyV1(
        candidate_id="CROSS_DECK|P1|cap.aura|cap.draw|DIRECTIONAL_BINARY",
        candidate_identity_digest=bytes(32),
        source_instance_id="si.v1/candidate/0",
    )


def _witness(row: int) -> HostRealizationWitnessV1:
    return HostRealizationWitnessV1(
        discovery_mapping_ref=_ref(
            "rev3_card_requirement_map",
            "derived/Card_Requirement_Map_REV3.csv",
            row,
        ),
        deck_row_ref=_ref(
            "rev3_deck_row_source_resolution",
            "inputs/deck_row_source_resolution_REV3.csv",
            row,
        ),
        osi_ref=_ref(
            "rev3_osi_source_records",
            "source/raw/oracle_cards_selected_REV3.jsonl",
            row,
        ),
        b2_assignment_refs=(
            _ref(
                "b2_classifications",
                "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
                row,
            ),
        ),
    )


def _single_member_claim() -> CrossDeckHostBindingClaimV1:
    member = _member()
    host = DiscoveryHostRefV1("rev3_deck", "Token Triumph")
    discovery = CrossDeckParticipantDiscoveryHostBindingV1(
        member_key=member,
        participant_position=0,
        participant_ref="cap.aura",
        discovery_side="rev3_left_family",
        discovery_host=host,
        mapping_evidence_refs=(
            _ref(
                "rev3_card_requirement_map",
                "derived/Card_Requirement_Map_REV3.csv",
                1,
            ),
        ),
    )
    realization = ParticipantHostRealizationV1(
        member_key=member,
        participant_position=0,
        participant_ref="cap.aura",
        host=host,
        witnesses=(_witness(1),),
    )
    return CrossDeckHostBindingClaimV1(
        member_key=member,
        discovery_bindings=(discovery,),
        participant_host_realizations=(realization,),
        observed_host_relationship="same_host",
    )


class HostBindingContractTests(unittest.TestCase):
    def test_member_key_has_exact_three_field_cbor_shape(self) -> None:
        member = _member()

        self.assertEqual(
            member.to_cbor(),
            [
                "CROSS_DECK|P1|cap.aura|cap.draw|DIRECTIONAL_BINARY",
                bytes(32),
                "si.v1/candidate/0",
            ],
        )
        with self.assertRaises(AttributeError):
            member.candidate_id = "mutated"  # type: ignore[misc]

    def test_witness_is_one_correlated_four_field_join(self) -> None:
        witness = _witness(4)

        self.assertEqual(len(witness.to_cbor()), 4)
        self.assertNotEqual(witness.to_cbor()[0], witness.to_cbor()[1])
        self.assertEqual(witness.to_cbor()[0][0], "rev3_card_requirement_map")
        self.assertEqual(witness.to_cbor()[1][0], "rev3_deck_row_source_resolution")
        self.assertEqual(witness.to_cbor()[2][0], "rev3_osi_source_records")
        self.assertEqual(witness.to_cbor()[3][0][0], "b2_classifications")

    def test_deck_row_evidence_cannot_reuse_discovery_mapping_role(self) -> None:
        mapping = _ref(
            "rev3_card_requirement_map",
            "derived/Card_Requirement_Map_REV3.csv",
            1,
        )
        osi = _ref(
            "rev3_osi_source_records",
            "source/raw/oracle_cards_selected_REV3.jsonl",
            1,
        )
        b2 = _ref(
            "b2_classifications",
            "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
            1,
        )

        with self.assertRaises(HostBindingContractError):
            HostRealizationWitnessV1(mapping, mapping, osi, (b2,))

    def test_witnesses_must_be_nonempty_canonical_and_duplicate_free(self) -> None:
        member = _member()
        discovery = CrossDeckParticipantDiscoveryHostBindingV1(
            member_key=member,
            participant_position=0,
            participant_ref="cap.aura",
            discovery_side="rev3_left_family",
            discovery_host=DiscoveryHostRefV1("rev3_deck", "Token Triumph"),
            mapping_evidence_refs=(
                _ref(
                    "rev3_card_requirement_map",
                    "derived/Card_Requirement_Map_REV3.csv",
                    1,
                ),
            ),
        )

        with self.assertRaises(HostBindingContractError):
            ParticipantHostRealizationV1(
                member_key=member,
                participant_position=0,
                participant_ref="cap.aura",
                host=DiscoveryHostRefV1("rev3_deck", "Token Triumph"),
                witnesses=(),
            )

        first = _witness(1)
        second = _witness(2)
        with self.assertRaises(HostBindingContractError):
            ParticipantHostRealizationV1(
                member_key=member,
                participant_position=0,
                participant_ref="cap.aura",
                host=DiscoveryHostRefV1("rev3_deck", "Token Triumph"),
                witnesses=(second, first),
            )

        with self.assertRaises(HostBindingContractError):
            ParticipantHostRealizationV1(
                member_key=member,
                participant_position=0,
                participant_ref="cap.aura",
                host=DiscoveryHostRefV1("rev3_deck", "Token Triumph"),
                witnesses=(first, first),
            )

        self.assertEqual(discovery.discovery_host.host_id, "Token Triumph")

    def test_realization_host_must_equal_discovery_host(self) -> None:
        member = _member()
        discovery = CrossDeckParticipantDiscoveryHostBindingV1(
            member_key=member,
            participant_position=0,
            participant_ref="cap.aura",
            discovery_side="rev3_left_family",
            discovery_host=DiscoveryHostRefV1("rev3_deck", "Token Triumph"),
            mapping_evidence_refs=(
                _ref(
                    "rev3_card_requirement_map",
                    "derived/Card_Requirement_Map_REV3.csv",
                    1,
                ),
            ),
        )
        realization = ParticipantHostRealizationV1(
            member_key=member,
            participant_position=0,
            participant_ref="cap.aura",
            host=DiscoveryHostRefV1("rev3_deck", "Grave Danger"),
            witnesses=(_witness(1),),
        )

        with self.assertRaises(HostBindingContractError):
            CrossDeckHostBindingClaimV1(
                member_key=member,
                discovery_bindings=(discovery,),
                participant_host_realizations=(realization,),
                observed_host_relationship="same_host",
            )

    def test_claim_is_member_atomic_and_identity_is_namespaced(self) -> None:
        member = _member()
        token = DiscoveryHostRefV1("rev3_deck", "Token Triumph")
        grave = DiscoveryHostRefV1("rev3_deck", "Grave Danger")
        discovery = (
            CrossDeckParticipantDiscoveryHostBindingV1(
                member_key=member,
                participant_position=0,
                participant_ref="cap.aura",
                discovery_side="rev3_left_family",
                discovery_host=token,
                mapping_evidence_refs=(
                    _ref(
                        "rev3_card_requirement_map",
                        "derived/Card_Requirement_Map_REV3.csv",
                        1,
                    ),
                ),
            ),
            CrossDeckParticipantDiscoveryHostBindingV1(
                member_key=member,
                participant_position=1,
                participant_ref="cap.draw",
                discovery_side="rev3_right_family",
                discovery_host=grave,
                mapping_evidence_refs=(
                    _ref(
                        "rev3_card_requirement_map",
                        "derived/Card_Requirement_Map_REV3.csv",
                        2,
                    ),
                ),
            ),
        )
        claim = CrossDeckHostBindingClaimV1(
            member_key=member,
            discovery_bindings=discovery,
            participant_host_realizations=(
                ParticipantHostRealizationV1(
                    member_key=member,
                    participant_position=0,
                    participant_ref="cap.aura",
                    host=token,
                    witnesses=(_witness(1),),
                ),
                ParticipantHostRealizationV1(
                    member_key=member,
                    participant_position=1,
                    participant_ref="cap.draw",
                    host=grave,
                    witnesses=(_witness(2),),
                ),
            ),
            observed_host_relationship="cross_host",
        )

        self.assertEqual(claim.to_cbor()[0], member.to_cbor())
        self.assertEqual(
            claim.identity().as_text(),
            "hbc.v1/0cc97a67b98c685c1715a79e9c54243d9739883fc2dd24a7de25dc4f4892139b",
        )

    def test_application_link_targets_semantic_id_not_record_id(self) -> None:
        link = ApplicationHostBindingV1(
            application_kind="relation_application",
            application_semantic_id="rpa.v1/" + "01" * 32,
            host_binding_claim_ids=("hbc.v1/" + "02" * 32,),
        )

        self.assertNotIn("record_id", link.to_wire())
        self.assertEqual(link.to_cbor()[0], "relation_application")

    def test_claim_record_uses_v2_event_and_recomputes_its_own_identity(self) -> None:
        member = _member()
        token = DiscoveryHostRefV1("rev3_deck", "Token Triumph")
        discovery = CrossDeckParticipantDiscoveryHostBindingV1(
            member_key=member,
            participant_position=0,
            participant_ref="cap.aura",
            discovery_side="rev3_left_family",
            discovery_host=token,
            mapping_evidence_refs=(
                _ref(
                    "rev3_card_requirement_map",
                    "derived/Card_Requirement_Map_REV3.csv",
                    1,
                ),
            ),
        )
        claim = CrossDeckHostBindingClaimV1(
            member_key=member,
            discovery_bindings=(discovery,),
            participant_host_realizations=(
                ParticipantHostRealizationV1(
                    member_key=member,
                    participant_position=0,
                    participant_ref="cap.aura",
                    host=token,
                    witnesses=(_witness(1),),
                ),
            ),
            observed_host_relationship="same_host",
        )
        record = CrossDeckHostBindingClaimRecordV1(
            claim=claim,
            acceptance_event_ref=HostBindingAcceptanceEventRefV2(
                path="sources/m2_5/authorities/review_acceptance_events/v2/" + "03" * 32 + ".json",
                raw_sha256=bytes(32),
                event_id="ae.v2/" + "03" * 32,
            ),
        )

        wire = record.to_wire()

        from mtgml.host_binding import host_binding_claim_record_from_wire

        parsed = host_binding_claim_record_from_wire(wire)
        self.assertEqual(parsed.claim.identity(), claim.identity())
        self.assertEqual(parsed.record_identity(), record.record_identity())
        self.assertEqual(wire["acceptance"]["decision"], "human_accepted")

    def test_v2_acceptance_event_has_its_own_namespaced_identity(self) -> None:
        model_binding = HostBindingSourceBindingV2(
            artifact_role="declared_model",
            path="sources/m2_5/closures/C/declared_interaction_model.v2.json",
            schema_or_null="manafold.m2.5.c.declared-interaction-model.v2",
            raw_sha256=bytes(32),
        )
        roster_binding = HostBindingSourceBindingV2(
            artifact_role="reviewer_roster_leaf",
            path="sources/m2_5/authorities/reviewer_rosters/v1/" + "00" * 32 + ".json",
            schema_or_null="manafold.m2.5.c.reviewer-roster.v1",
            raw_sha256=bytes(32),
        )
        deck_binding = HostBindingSourceBindingV2(
            artifact_role="rev3_deck_row_source_resolution",
            path="inputs/deck_row_source_resolution_REV3.csv",
            schema_or_null=None,
            raw_sha256=bytes(32),
        )
        event_input = HostBindingAcceptanceEventInputV2(
            subject_kind="cross_deck_host_binding_claim_record_v1",
            subject_payload_digest=bytes(32),
            reviewer_roster_ref=ReviewerRosterRefV1(
                path=roster_binding.path,
                schema=roster_binding.schema_or_null,
                raw_sha256=bytes(32),
            ),
            reviewer_role_bindings=(ReviewerRoleBindingV1("alice", ("project_owner",)),),
            review_mode=ReviewMode.SOLO_SEPARATE_SELF_REVIEW,
            checklist_id="cross-deck-host-binding-review-checklist.v1",
            source_binding_digests=(model_binding, roster_binding, deck_binding),
            review_evidence_refs=(
                AcceptanceEvidenceRefV1(
                    "docs/review/host-binding.md",
                    bytes(32),
                    ("whole_artifact", None),
                ),
            ),
        )
        leaf = HostBindingAcceptanceEventLeafV2.from_input(event_input)

        self.assertEqual(
            leaf.event_id.as_text(),
            "ae.v2/f6abfd98ab68cd735ea27d0e25a71049609c29a085e2cd99c656ce24eaaa114f",
        )
        self.assertEqual(leaf.to_wire()["schema"], "manafold.m2.5.c.review-acceptance-event.v2")
        event_schema = json.loads(
            (ROOT / "schemas" / "review-acceptance-event.v2.schema.json").read_text(
                encoding="utf-8"
            )
        )
        jsonschema.Draft202012Validator(event_schema).validate(leaf.to_wire())
        from mtgml.host_binding import host_binding_acceptance_event_from_wire

        parsed = host_binding_acceptance_event_from_wire(leaf.to_wire())
        self.assertEqual(parsed.identity(), event_input.identity())

    def test_claim_record_and_supersession_use_separate_same_family_identities(self) -> None:
        claim = _single_member_claim()
        first_event_ref = HostBindingAcceptanceEventRefV2(
            path="sources/m2_5/authorities/review_acceptance_events/v2/" + "04" * 32 + ".json",
            raw_sha256=bytes(32),
            event_id="ae.v2/" + "04" * 32,
        )
        second_event_ref = HostBindingAcceptanceEventRefV2(
            path="sources/m2_5/authorities/review_acceptance_events/v2/" + "05" * 32 + ".json",
            raw_sha256=bytes(32),
            event_id="ae.v2/" + "05" * 32,
        )
        record = CrossDeckHostBindingClaimRecordV1(claim, first_event_ref)
        first_supersession = CrossDeckHostBindingClaimSupersessionV1(
            superseded_record_id=record.record_identity(),
            replacement_record_id=None,
            reason_code="authority_revocation",
            source_evidence_refs=(
                _ref(
                    "rev3_card_requirement_map",
                    "derived/Card_Requirement_Map_REV3.csv",
                    1,
                ),
            ),
            acceptance_event_ref=first_event_ref,
        )
        second_supersession = CrossDeckHostBindingClaimSupersessionV1(
            superseded_record_id=record.record_identity(),
            replacement_record_id=None,
            reason_code="authority_revocation",
            source_evidence_refs=(
                _ref(
                    "rev3_card_requirement_map",
                    "derived/Card_Requirement_Map_REV3.csv",
                    1,
                ),
            ),
            acceptance_event_ref=second_event_ref,
        )

        self.assertTrue(record.record_identity().as_text().startswith("hbcr.v1/"))
        self.assertTrue(first_supersession.identity().as_text().startswith("hbcs.v1/"))
        self.assertEqual(first_supersession.identity(), second_supersession.identity())
        second_record = CrossDeckHostBindingClaimRecordV1(claim, second_event_ref)
        self.assertNotEqual(record.record_identity(), second_record.record_identity())


class HostBindingSchemaTests(unittest.TestCase):
    def _schema(self) -> dict[str, object]:
        return json.loads(
            (ROOT / "schemas" / "interaction-review-authority.v2.schema.json").read_text(
                encoding="utf-8"
            )
        )

    def _valid_root(self) -> dict[str, object]:
        binding = {
            "artifact_role": "base_authority_v1",
            "path": "sources/m2_5/authorities/interaction_review_authority.v1.json",
            "schema_or_null": "manafold.m2.5.c.interaction-review-authority.v1",
            "raw_sha256": "00" * 32,
        }
        return {
            "schema": HOST_BINDING_AUTHORITY_SCHEMA_V2,
            "base_authority_v1_binding": binding,
            "source_bindings": [binding],
            "cross_deck_host_binding_claim_records": [],
            "cross_deck_host_binding_claim_supersession_records": [],
            "application_host_bindings": [],
        }

    def test_v2_root_is_closed_and_accepts_empty_underlay(self) -> None:
        schema = self._schema()
        jsonschema.Draft202012Validator(schema).validate(self._valid_root())
        self.assertEqual(schema["properties"]["schema"]["const"], HOST_BINDING_AUTHORITY_SCHEMA_V2)
        self.assertIs(schema["additionalProperties"], False)

    def test_v1_root_cannot_be_used_for_host_binding_records(self) -> None:
        schema = self._schema()
        document = self._valid_root()
        document["schema"] = "manafold.m2.5.c.interaction-review-authority.v1"
        document["application_host_bindings"] = [
            {
                "application_kind": "relation_application",
                "application_semantic_id": "rpa.v1/" + "01" * 32,
                "host_binding_claim_ids": ["hbc.v1/" + "02" * 32],
            }
        ]

        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(document)

    def test_unknown_v2_source_role_is_rejected(self) -> None:
        schema = self._schema()
        document = self._valid_root()
        document["source_bindings"] = [
            {
                "artifact_role": "invented_source",
                "path": "derived/unknown.csv",
                "schema_or_null": None,
                "raw_sha256": "00" * 32,
            }
        ]

        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(document)

    def test_claim_wire_rejects_application_record_id_and_non_atomic_member_shape(self) -> None:
        schema = self._schema()
        root = self._valid_root()
        claim = _single_member_claim()
        record = CrossDeckHostBindingClaimRecordV1(
            claim=claim,
            acceptance_event_ref=HostBindingAcceptanceEventRefV2(
                path="sources/m2_5/authorities/review_acceptance_events/v2/" + "05" * 32 + ".json",
                raw_sha256=bytes(32),
                event_id="ae.v2/" + "05" * 32,
            ),
        )
        root["cross_deck_host_binding_claim_records"] = [record.to_wire()]
        jsonschema.Draft202012Validator(schema).validate(root)

        mutated = json.loads(json.dumps(root))
        mutated["cross_deck_host_binding_claim_records"][0]["claim"]["application_record_id"] = (
            "rpar.v1/" + "06" * 32
        )
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(mutated)


if __name__ == "__main__":
    unittest.main()
