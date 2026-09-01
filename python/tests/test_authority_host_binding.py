from __future__ import annotations

import base64
import csv
import hashlib
import io
import json
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from authority_host_binding import (
    HostBindingSourceError,
    HostBindingSourceResolver,
)
from authority_source_resolver import (
    CANDIDATE_IDENTITY_DOMAIN,
    CANDIDATE_IDENTITY_SCHEMA,
    CANDIDATE_UNIVERSE_PATH,
    CANDIDATE_UNIVERSE_SCHEMA,
    REV3_CENSUS_MEMBER,
    REV3_SOURCE_COLUMNS,
    AuthoritySourceResolver,
    ResolutionError,
    ResolutionStatus,
    Rev3ArchiveStore,
)
from mtgml.host_binding import (
    ApplicationMemberKeyV1,
    CrossDeckParticipantDiscoveryHostBindingV1,
    DiscoveryHostRefV1,
    HostBindingEvidenceRefV2,
    HostBindingSourceBindingV2,
    HostRealizationWitnessV1,
    ParticipantHostRealizationV1,
)
from mtgml.persistence import encode_canonical, encode_envelope, hash_envelope

MAP_PATH = "derived/Card_Requirement_Map_REV3.csv"
OSI_PATH = "source/raw/oracle_cards_selected_REV3.jsonl"
B2_PATH = "sources/m2_5/closures/B2/card_semantic_classifications.v1.json"
B2_SCHEMA = "manafold.m2.5.b2.card-semantic-classifications.v1"


def _sha(raw: bytes) -> bytes:
    return hashlib.sha256(raw).digest()


def _zip_bytes(members: dict[str, bytes]) -> bytes:
    entries = [
        {"path": path, "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}
        for path, raw in members.items()
    ]
    manifest = {
        "schema": "manafold.m2.5.rev3.package-manifest.v1",
        "manifest_excludes_self": True,
        "manifest_excluded_paths": [],
        "entries": entries,
    }
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_STORED) as archive:
        for path, raw in members.items():
            archive.writestr(path, raw)
        archive.writestr(
            "Manafold_M2_5_Package_Manifest_REV3.json",
            (json.dumps(manifest, separators=(",", ":")) + "\n").encode("utf-8"),
        )
    return output.getvalue()


class AuthorityHostBindingSourceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.repo = Path(self.tempdir.name) / "repo"
        self.repo.mkdir()
        self.map_raw = (
            b"deck_row_id,deck_id,oracle_semantic_identity,requirement_id,provenance,ranking_eligible\n"
            b"token:line-1,Token Triumph,osi-a,cap.aura,INHERITED_REV2_CANDIDATE,False\n"
            b"grave:line-2,Grave Danger,osi-b,cap.draw,INHERITED_REV2_CANDIDATE,False\n"
        )
        self.osi_raw = (
            json.dumps({"oracle_id": "osi-a", "name": "Token Aura"})
            + "\n"
            + json.dumps({"oracle_id": "osi-b", "name": "Grave Draw"})
            + "\n"
        ).encode("utf-8")
        archive_raw = _zip_bytes({MAP_PATH: self.map_raw, OSI_PATH: self.osi_raw})
        self.archive = Rev3ArchiveStore.from_bytes(
            archive_raw,
            hashlib.sha256(archive_raw).hexdigest(),
        )
        b2_document = {
            "schema": B2_SCHEMA,
            "classifications": [
                {
                    "oracle_semantic_identity": "osi-a",
                    "requirement_assignments": [
                        {"requirement_family_id": "cap.aura"},
                    ],
                },
                {
                    "oracle_semantic_identity": "osi-b",
                    "requirement_assignments": [
                        {"requirement_family_id": "cap.draw"},
                    ],
                },
            ],
        }
        b2_path = self.repo / Path(*B2_PATH.split("/"))
        b2_path.parent.mkdir(parents=True, exist_ok=True)
        b2_raw = (json.dumps(b2_document, separators=(",", ":")) + "\n").encode("utf-8")
        b2_path.write_bytes(b2_raw)
        self.resolver = HostBindingSourceResolver(
            AuthoritySourceResolver(self.repo, rev3_archive=self.archive)
        )
        self.member = ApplicationMemberKeyV1("candidate", bytes(32), "si.v1/candidate/0")

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def _ref(
        self,
        role: str,
        path: str,
        raw: bytes,
        locator: tuple[str, str | int | None],
        schema: str | None = None,
    ) -> HostBindingEvidenceRefV2:
        return HostBindingEvidenceRefV2(
            artifact_role=role,
            path=path,
            schema_or_null=schema,
            raw_sha256=_sha(raw),
            locator=locator,
        )

    def _binding(
        self, position: int, ref: str, host: str, row: int
    ) -> CrossDeckParticipantDiscoveryHostBindingV1:
        return CrossDeckParticipantDiscoveryHostBindingV1(
            member_key=self.member,
            participant_position=position,
            participant_ref=ref,
            discovery_side=("rev3_left_family" if position == 0 else "rev3_right_family"),
            discovery_host=DiscoveryHostRefV1("rev3_deck", host),
            mapping_evidence_refs=(
                self._ref("rev3_card_requirement_map", MAP_PATH, self.map_raw, ("csv_row", row)),
            ),
        )

    def _witness(self, row: int, osi_line: int, b2_index: int) -> HostRealizationWitnessV1:
        return HostRealizationWitnessV1(
            discovery_mapping_ref=self._ref(
                "rev3_card_requirement_map", MAP_PATH, self.map_raw, ("csv_row", row)
            ),
            deck_row_ref=self._ref(
                "rev3_card_requirement_map", MAP_PATH, self.map_raw, ("csv_row", row)
            ),
            osi_ref=self._ref(
                "rev3_osi_source_records", OSI_PATH, self.osi_raw, ("jsonl_line", osi_line)
            ),
            b2_assignment_refs=(
                self._ref(
                    "b2_classifications",
                    B2_PATH,
                    (self.repo / Path(*B2_PATH.split("/"))).read_bytes(),
                    ("json_pointer", f"/classifications/{b2_index}"),
                    B2_SCHEMA,
                ),
            ),
        )

    def test_exact_correlated_witness_resolves_to_the_discovery_host(self) -> None:
        binding = self._binding(0, "cap.aura", "Token Triumph", 0)
        realization = ParticipantHostRealizationV1(
            member_key=self.member,
            participant_position=0,
            participant_ref="cap.aura",
            host=DiscoveryHostRefV1("rev3_deck", "Token Triumph"),
            witnesses=(self._witness(0, 0, 0),),
        )

        resolved = self.resolver.resolve_participant_realization(binding, realization)

        self.assertEqual(resolved.host.host_id, "Token Triumph")
        self.assertEqual(resolved.witnesses[0].deck_row["oracle_semantic_identity"], "osi-a")
        self.assertEqual(resolved.witnesses[0].osi_record["oracle_id"], "osi-a")
        self.assertEqual(
            resolved.witnesses[0].b2_assignments[0]["requirement_family_id"], "cap.aura"
        )

    def test_witness_with_wrong_host_fails_closed(self) -> None:
        binding = self._binding(0, "cap.aura", "Token Triumph", 0)
        realization = ParticipantHostRealizationV1(
            member_key=self.member,
            participant_position=0,
            participant_ref="cap.aura",
            host=DiscoveryHostRefV1("rev3_deck", "Grave Danger"),
            witnesses=(self._witness(0, 0, 0),),
        )

        with self.assertRaises(HostBindingSourceError):
            self.resolver.resolve_participant_realization(binding, realization)

    def test_witness_with_wrong_osi_or_b2_assignment_fails_closed(self) -> None:
        binding = self._binding(0, "cap.aura", "Token Triumph", 0)
        wrong_osi = ParticipantHostRealizationV1(
            member_key=self.member,
            participant_position=0,
            participant_ref="cap.aura",
            host=DiscoveryHostRefV1("rev3_deck", "Token Triumph"),
            witnesses=(self._witness(0, 1, 0),),
        )
        wrong_b2 = ParticipantHostRealizationV1(
            member_key=self.member,
            participant_position=0,
            participant_ref="cap.aura",
            host=DiscoveryHostRefV1("rev3_deck", "Token Triumph"),
            witnesses=(self._witness(0, 0, 1),),
        )

        with self.assertRaises(HostBindingSourceError):
            self.resolver.resolve_participant_realization(binding, wrong_osi)
        with self.assertRaises(HostBindingSourceError):
            self.resolver.resolve_participant_realization(binding, wrong_b2)

    def test_full_b2_classification_record_resolves_one_assignment(self) -> None:
        binding = self._binding(0, "cap.aura", "Token Triumph", 0)
        b2_document = {
            "schema": B2_SCHEMA,
            "classifications": [
                {
                    "oracle_semantic_identity": "osi-a",
                    "requirement_assignments": [
                        {"requirement_family_id": "cap.aura"},
                    ],
                }
            ],
        }
        b2_raw = (json.dumps(b2_document, separators=(",", ":")) + "\n").encode("utf-8")
        b2_file = self.repo / Path(*B2_PATH.split("/"))
        b2_file.write_bytes(b2_raw)
        map_ref = self._ref("rev3_card_requirement_map", MAP_PATH, self.map_raw, ("csv_row", 0))
        osi_ref = self._ref("rev3_osi_source_records", OSI_PATH, self.osi_raw, ("jsonl_line", 0))
        b2_ref = self._ref(
            "b2_classifications",
            B2_PATH,
            b2_raw,
            ("json_pointer", "/classifications/0"),
            B2_SCHEMA,
        )
        witness = HostRealizationWitnessV1(map_ref, map_ref, osi_ref, (b2_ref,))
        realization = ParticipantHostRealizationV1(
            self.member,
            0,
            "cap.aura",
            DiscoveryHostRefV1("rev3_deck", "Token Triumph"),
            (witness,),
        )

        resolved = self.resolver.resolve_participant_realization(binding, realization)

        self.assertEqual(
            resolved.witnesses[0].b2_assignments[0]["oracle_semantic_identity"],
            "osi-a",
        )
        self.assertEqual(
            resolved.witnesses[0].b2_assignments[0]["requirement_family_id"],
            "cap.aura",
        )

    def test_claim_rebinds_to_exact_candidate_source_and_pair_hosts(self) -> None:
        census_path = REV3_CENSUS_MEMBER
        pair_path = "derived/Pair_Requirement_Aggregates_REV3.json"
        census_row = [
            "candidate",
            "interaction-model.v1",
            "CROSS_DECK",
            "P1",
            "cap.aura",
            "cap.draw",
            "DIRECTIONAL_BINARY",
            "AMBIGUOUS_REQUIRES_REVIEW",
            "candidate classification authority or interaction trigger is not terminally reviewed",
            '["cap.aura", "cap.draw"]',
        ]
        census_buffer = io.StringIO(newline="")
        writer = csv.writer(census_buffer, lineterminator="\n")
        writer.writerow(REV3_SOURCE_COLUMNS)
        writer.writerow(census_row)
        census_raw = census_buffer.getvalue().encode("utf-8")
        pair_raw = b'{"pairs":{"P1":{"pair":["Token Triumph","Grave Danger"]}}}\n'
        archive_raw = _zip_bytes(
            {
                census_path: census_raw,
                MAP_PATH: self.map_raw,
                OSI_PATH: self.osi_raw,
                pair_path: pair_raw,
            }
        )
        archive = Rev3ArchiveStore.from_bytes(
            archive_raw,
            hashlib.sha256(archive_raw).hexdigest(),
        )

        model_path = (
            self.repo / "sources" / "m2_5" / "closures" / "C" / "declared_interaction_model.v2.json"
        )
        model_path.parent.mkdir(parents=True, exist_ok=True)
        model_raw = (
            ROOT / "sources" / "m2_5" / "closures" / "C" / "declared_interaction_model.v2.json"
        ).read_bytes()
        model_path.write_bytes(model_raw)
        candidate_binding = {
            "kind": "rev3",
            "archive_member": census_path,
            "archive_member_sha256": hashlib.sha256(census_raw).hexdigest(),
            "row_ordinal": 0,
            "source_columns": list(REV3_SOURCE_COLUMNS),
            "source_values": census_row,
        }
        participant_refs = [
            {"participant_kind": "requirement_family", "semantic_ref": "cap.aura"},
            {"participant_kind": "requirement_family", "semantic_ref": "cap.draw"},
        ]
        participant_payload = [
            [[[item["participant_kind"], None], item["semantic_ref"]]] for item in participant_refs
        ]
        participant_payload = [item[0] for item in participant_payload]
        candidate_identity_payload = [
            ["rev3", None],
            ["cross_deck", None],
            ["directional_binary", None],
            participant_payload,
            ["cap.aura", "cap.draw"],
            [
                ["rev3", None],
                [
                    census_path,
                    hashlib.sha256(census_raw).digest(),
                    0,
                    list(REV3_SOURCE_COLUMNS),
                    census_row,
                ],
            ],
        ]
        identity_digest = hash_envelope(
            encode_envelope(
                CANDIDATE_IDENTITY_DOMAIN,
                CANDIDATE_IDENTITY_SCHEMA,
                encode_canonical(candidate_identity_payload),
            )
        )
        candidate_identity = {
            "algorithm_id": "sha-256",
            "digest_hex": identity_digest.hex(),
            "envelope_id": "mtgml.digest-envelope.v1",
            "input_schema_id": CANDIDATE_IDENTITY_SCHEMA,
            "payload_codec_id": "mtgml.canonical-cbor.v1",
            "semantic_domain": CANDIDATE_IDENTITY_DOMAIN,
        }
        candidate = {
            "candidate_id": "candidate",
            "candidate_identity": candidate_identity,
            "source_origin": "rev3",
            "scope": "cross_deck",
            "relation": "directional_binary",
            "participant_refs": participant_refs,
            "supporting_requirement_ids": ["cap.aura", "cap.draw"],
            "source_binding": candidate_binding,
            "reconciliation_status": "unchanged",
            "reconciliation_reason": "host-binding source test",
        }
        source_instance = {
            "source_instance_id": "si.v1/"
            + base64.urlsafe_b64encode(b"candidate").decode().rstrip("=")
            + "/0",
            "candidate_id": "candidate",
            "source_binding": candidate_binding,
            "participant_bindings": [
                {"role": "ordered_participant", "participant_ref": item}
                for item in participant_refs
            ],
            "source_context": {
                key: "not_applicable"
                for key in (
                    "zone",
                    "visibility",
                    "timing",
                    "temporal_order",
                    "source_affected_relation",
                    "control_ownership_relation",
                    "replacement_layer_relation",
                    "trigger_lki_relation",
                    "information_relation",
                    "decision_actor_relation",
                )
            },
        }
        universe = {
            "schema": CANDIDATE_UNIVERSE_SCHEMA,
            "model_id": "declared-interaction-model.v2",
            "input_bindings": {
                "declared_model": {
                    "path": "sources/m2_5/closures/C/declared_interaction_model.v2.json",
                    "raw_sha256": hashlib.sha256(model_raw).hexdigest(),
                },
                "review_additions": {
                    "path": "sources/m2_5/closures/C/interaction_review_additions.v2.json",
                    "raw_sha256": "11" * 32,
                },
                "rev3_candidate_source": {
                    "archive_member": census_path,
                    "archive_member_sha256": hashlib.sha256(census_raw).hexdigest(),
                    "source_package_sha256": hashlib.sha256(archive_raw).hexdigest(),
                },
                "b2_artifacts": [
                    {
                        "path": "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
                        "raw_sha256": "22" * 32,
                    },
                    {
                        "path": "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
                        "raw_sha256": "33" * 32,
                    },
                    {
                        "path": "sources/m2_5/closures/B2/classification_closure.v1.json",
                        "raw_sha256": "44" * 32,
                    },
                ],
                "b1_final_artifacts": [
                    {
                        "path": "sources/m2_5/closures/B1/official_authority_citations.v3.json",
                        "raw_sha256": "55" * 32,
                    },
                    {
                        "path": (
                            "sources/m2_5/closures/B1/official_authority_citation_closure.v2.json"
                        ),
                        "raw_sha256": "66" * 32,
                    },
                ],
            },
            "candidate_count": 1,
            "candidate_reconciliation_counts": {
                "unchanged": 1,
                "stale_rev3_candidate": 0,
                "removed_not_interaction": 0,
                "merged_semantic_duplicate": 0,
                "new_targeted_higher_order_candidate": 0,
                "new_b2_derived": 0,
            },
            "source_instance_count": 1,
            "candidates": [candidate],
            "source_instances": [source_instance],
        }
        universe_raw = (json.dumps(universe, separators=(",", ":")) + "\n").encode("utf-8")
        universe_path = self.repo / Path(*CANDIDATE_UNIVERSE_PATH.split("/"))
        universe_path.parent.mkdir(parents=True, exist_ok=True)
        universe_path.write_bytes(universe_raw)
        pair_ref = HostBindingSourceBindingV2(
            "rev3_pair_aggregates",
            pair_path,
            None,
            hashlib.sha256(pair_raw).digest(),
        )
        candidate_ref = HostBindingSourceBindingV2(
            "candidate_universe",
            CANDIDATE_UNIVERSE_PATH,
            CANDIDATE_UNIVERSE_SCHEMA,
            hashlib.sha256(universe_raw).digest(),
        )

        m = ApplicationMemberKeyV1(
            "candidate", identity_digest, source_instance["source_instance_id"]
        )
        token = DiscoveryHostRefV1("rev3_deck", "Token Triumph")
        grave = DiscoveryHostRefV1("rev3_deck", "Grave Danger")
        map_a = HostBindingEvidenceRefV2(
            "rev3_card_requirement_map", MAP_PATH, None, _sha(self.map_raw), ("csv_row", 0)
        )
        map_b = HostBindingEvidenceRefV2(
            "rev3_card_requirement_map", MAP_PATH, None, _sha(self.map_raw), ("csv_row", 1)
        )
        osi_a = HostBindingEvidenceRefV2(
            "rev3_osi_source_records", OSI_PATH, None, _sha(self.osi_raw), ("jsonl_line", 0)
        )
        osi_b = HostBindingEvidenceRefV2(
            "rev3_osi_source_records", OSI_PATH, None, _sha(self.osi_raw), ("jsonl_line", 1)
        )
        b2_file_raw = (
            b'{"classifications":['
            b'{"oracle_semantic_identity":"osi-a","requirement_assignments":['
            b'{"requirement_family_id":"cap.aura"}]},'
            b'{"oracle_semantic_identity":"osi-b","requirement_assignments":['
            b'{"requirement_family_id":"cap.draw"}]}],'
            b'"schema":"manafold.m2.5.b2.card-semantic-classifications.v1"}\n'
        )
        b2_file = self.repo / Path(*B2_PATH.split("/"))
        b2_file.parent.mkdir(parents=True, exist_ok=True)
        b2_file.write_bytes(b2_file_raw)
        b2_a = HostBindingEvidenceRefV2(
            "b2_classifications",
            B2_PATH,
            B2_SCHEMA,
            _sha(b2_file_raw),
            ("json_pointer", "/classifications/0"),
        )
        b2_b = HostBindingEvidenceRefV2(
            "b2_classifications",
            B2_PATH,
            B2_SCHEMA,
            _sha(b2_file_raw),
            ("json_pointer", "/classifications/1"),
        )
        claim = __import__(
            "mtgml.host_binding", fromlist=["CrossDeckHostBindingClaimV1"]
        ).CrossDeckHostBindingClaimV1(
            m,
            (
                CrossDeckParticipantDiscoveryHostBindingV1(
                    m, 0, "cap.aura", "rev3_left_family", token, (map_a,)
                ),
                CrossDeckParticipantDiscoveryHostBindingV1(
                    m, 1, "cap.draw", "rev3_right_family", grave, (map_b,)
                ),
            ),
            (
                ParticipantHostRealizationV1(
                    m,
                    0,
                    "cap.aura",
                    token,
                    (HostRealizationWitnessV1(map_a, map_a, osi_a, (b2_a,)),),
                ),
                ParticipantHostRealizationV1(
                    m,
                    1,
                    "cap.draw",
                    grave,
                    (HostRealizationWitnessV1(map_b, map_b, osi_b, (b2_b,)),),
                ),
            ),
            "cross_host",
        )
        source_resolver = HostBindingSourceResolver(
            AuthoritySourceResolver(self.repo, rev3_archive=archive)
        )

        resolved = source_resolver.resolve_claim_for_member(
            claim,
            identity_digest,
            candidate_ref,
            pair_ref,
        )

        self.assertEqual(
            [item.host.host_id for item in resolved], ["Token Triumph", "Grave Danger"]
        )

    def test_missing_or_duplicate_mapping_witness_fails_closed(self) -> None:
        with self.assertRaises(ValueError):
            ParticipantHostRealizationV1(
                member_key=self.member,
                participant_position=0,
                participant_ref="cap.aura",
                host=DiscoveryHostRefV1("rev3_deck", "Token Triumph"),
                witnesses=(self._witness(0, 0, 0), self._witness(0, 0, 0)),
            )

        binding = self._binding(0, "cap.aura", "Token Triumph", 0)
        same_mapping_different_witness = ParticipantHostRealizationV1(
            self.member,
            0,
            "cap.aura",
            DiscoveryHostRefV1("rev3_deck", "Token Triumph"),
            (self._witness(0, 0, 0), self._witness(0, 1, 1)),
        )
        with self.assertRaises(HostBindingSourceError):
            self.resolver.resolve_participant_realization(
                binding,
                same_mapping_different_witness,
            )

    def test_missing_external_rev3_archive_remains_blocked(self) -> None:
        resolver = HostBindingSourceResolver(
            AuthoritySourceResolver(
                self.repo,
                rev3_archive_root=self.repo / "missing-rev3",
            )
        )
        binding = self._binding(0, "cap.aura", "Token Triumph", 0)
        realization = ParticipantHostRealizationV1(
            self.member,
            0,
            "cap.aura",
            DiscoveryHostRefV1("rev3_deck", "Token Triumph"),
            (self._witness(0, 0, 0),),
        )

        with self.assertRaises(ResolutionError) as context:
            resolver.resolve_participant_realization(binding, realization)
        self.assertEqual(context.exception.status, ResolutionStatus.BLOCKED)

    def test_missing_repository_b2_source_is_fail(self) -> None:
        resolver = HostBindingSourceResolver(
            AuthoritySourceResolver(
                self.repo,
                rev3_archive=self.archive,
            )
        )
        binding = self._binding(0, "cap.aura", "Token Triumph", 0)
        realization = ParticipantHostRealizationV1(
            self.member,
            0,
            "cap.aura",
            DiscoveryHostRefV1("rev3_deck", "Token Triumph"),
            (self._witness(0, 0, 0),),
        )
        b2_file = self.repo / Path(*B2_PATH.split("/"))
        b2_file.unlink()

        with self.assertRaises(ResolutionError) as context:
            resolver.resolve_participant_realization(binding, realization)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)


if __name__ == "__main__":
    unittest.main()
