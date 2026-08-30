from __future__ import annotations

import base64
import csv
import hashlib
import io
import json
import shutil
import sys
import tempfile
import unittest
import warnings
import zipfile
from collections.abc import Callable
from copy import deepcopy
from pathlib import Path
from typing import ClassVar, cast

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from authority_source_resolver import (
    ACCEPTANCE_EVENT_SCHEMA_V1,
    B2_CATALOG_PATH,
    B2_CATALOG_SCHEMA,
    B2_CLASSIFICATION_PATH,
    B2_CLASSIFICATION_SCHEMA,
    B2_CLOSURE_PATH,
    B2_CLOSURE_SCHEMA,
    REV3_CENSUS_MEMBER,
    AuthoritySourceResolver,
    B2ArtifactBindingsV1,
    ResolutionError,
    ResolutionStatus,
    ResolvedArtifact,
    Rev3ArchiveStore,
)
from mtgml.authority import (
    B2FamilyRefV1,
    ReviewerRosterRefV1,
    ReviewEventRefV1,
    SourceBindingDigestV1,
)
from mtgml.persistence import (
    CANONICAL_CBOR_ID,
    DIGEST_ENVELOPE_ID,
    SHA256_ID,
    encode_canonical,
    encode_envelope,
    hash_envelope,
)

CANDIDATE_UNIVERSE_PATH = "sources/m2_5/closures/C/interaction_candidate_universe.v2.json"
CANDIDATE_UNIVERSE_SCHEMA = "manafold.m2.5.c.interaction-candidate-universe.v2"
REV3_SOURCE_COLUMNS = (
    "candidate_id",
    "model_id",
    "scope",
    "pair_id",
    "left_family_id",
    "right_family_id",
    "relation",
    "disposition",
    "disposition_reason",
    "supporting_requirement_ids",
)


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def json_bytes(value: object) -> bytes:
    return (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode("utf-8")


def archive_bytes(
    entries: dict[str, bytes],
    *,
    manifest_entries: list[dict[str, object]] | None = None,
    duplicate_member: str | None = None,
) -> bytes:
    manifest = {
        "schema": "manafold.m2.5.rev3.package-manifest.v1",
        "entries": manifest_entries
        if manifest_entries is not None
        else [
            {"path": path, "bytes": len(raw), "sha256": digest(raw)}
            for path, raw in entries.items()
        ],
        "manifest_excluded_paths": [],
        "manifest_excludes_self": True,
    }
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w", compression=zipfile.ZIP_STORED) as archive:
        archive.writestr("Manafold_M2_5_Package_Manifest_REV3.json", json_bytes(manifest))
        for path, raw in entries.items():
            archive.writestr(path, raw)
            if path == duplicate_member:
                with warnings.catch_warnings():
                    warnings.simplefilter("ignore", UserWarning)
                    archive.writestr(path, raw)
    return buffer.getvalue()


class AuthoritySourceResolverTests(unittest.TestCase):
    CANDIDATE_ID = "CROSS_DECK|P1|family.a|family.b|DIRECTIONAL_BINARY"
    SOURCE_INSTANCE_ID = (
        "si.v1/"
        + base64.urlsafe_b64encode(CANDIDATE_ID.encode("utf-8")).decode("ascii").rstrip("=")
        + "/0"
    )
    CANDIDATE_IDENTITY: ClassVar[dict[str, str]] = {
        "algorithm_id": "sha-256",
        "digest_hex": "11" * 32,
        "envelope_id": "mtgml.digest-envelope.v1",
        "input_schema_id": "manafold.m2.5.c.candidate-identity-input.v1",
        "payload_codec_id": "mtgml.canonical-cbor.v1",
        "semantic_domain": "manafold.m2.5.c.candidate-identity.v1",
    }

    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.repo = Path(self.tempdir.name) / "repo"
        self.repo.mkdir()

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def write_repo(self, relative: str, raw: bytes) -> Path:
        path = self.repo / Path(*relative.split("/"))
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
        return path

    def assert_resolution_error(
        self,
        operation: Callable[[], object],
        status: ResolutionStatus,
        code: str,
    ) -> None:
        with self.assertRaises(ResolutionError) as context:
            operation()
        self.assertEqual(context.exception.status, status)
        self.assertEqual(context.exception.code, code)

    def _source_values(self, candidate_id: str | None = None) -> list[str]:
        return [
            candidate_id or self.CANDIDATE_ID,
            "interaction-model.v1",
            "CROSS_DECK",
            "P1",
            "family.a",
            "family.b",
            "DIRECTIONAL_BINARY",
            "AMBIGUOUS_REQUIRES_REVIEW",
            "candidate classification authority or interaction trigger is not terminally reviewed",
            '["family.a", "family.b"]',
        ]

    def _candidate_identity_for_test(self, candidate: dict[str, object]) -> dict[str, str]:
        binding = cast(dict[str, object], candidate["source_binding"])
        participant_payload = [
            [
                [
                    cast(str, cast(dict[str, str], ref)["participant_kind"]),
                    None,
                ],
                cast(str, cast(dict[str, str], ref)["semantic_ref"]),
            ]
            for ref in cast(list[dict[str, str]], candidate["participant_refs"])
        ]
        payload = [
            [cast(str, candidate["source_origin"]), None],
            [cast(str, candidate["scope"]), None],
            [cast(str, candidate["relation"]), None],
            participant_payload,
            list(cast(list[str], candidate["supporting_requirement_ids"])),
            [
                ["rev3", None],
                [
                    cast(str, binding["archive_member"]),
                    bytes.fromhex(cast(str, binding["archive_member_sha256"])),
                    cast(int, binding["row_ordinal"]),
                    list(cast(list[str], binding["source_columns"])),
                    list(cast(list[str], binding["source_values"])),
                ],
            ],
        ]
        digest_bytes = hash_envelope(
            encode_envelope(
                "manafold.m2.5.c.candidate-identity.v1",
                "manafold.m2.5.c.candidate-identity-input.v1",
                encode_canonical(payload),
            )
        )
        return {
            "algorithm_id": SHA256_ID,
            "digest_hex": digest_bytes.hex(),
            "envelope_id": DIGEST_ENVELOPE_ID,
            "input_schema_id": "manafold.m2.5.c.candidate-identity-input.v1",
            "payload_codec_id": CANONICAL_CBOR_ID,
            "semantic_domain": "manafold.m2.5.c.candidate-identity.v1",
        }

    def _census_bytes(self, rows: list[list[str]]) -> bytes:
        buffer = io.StringIO(newline="")
        writer = csv.writer(buffer, lineterminator="\n")
        writer.writerow(REV3_SOURCE_COLUMNS)
        writer.writerows(rows)
        return buffer.getvalue().encode("utf-8")

    def _candidate_record(
        self,
        *,
        candidate_id: str | None = None,
        source_binding: dict[str, object],
        candidate_identity: dict[str, str] | None = None,
    ) -> dict[str, object]:
        actual_candidate_id = candidate_id or self.CANDIDATE_ID
        return {
            "candidate_id": actual_candidate_id,
            "candidate_identity": deepcopy(candidate_identity or self.CANDIDATE_IDENTITY),
            "source_origin": "rev3",
            "scope": "cross_deck",
            "relation": "directional_binary",
            "participant_refs": [
                {"participant_kind": "requirement_family", "semantic_ref": "family.a"},
                {"participant_kind": "requirement_family", "semantic_ref": "family.b"},
            ],
            "supporting_requirement_ids": ["family.a", "family.b"],
            "source_binding": source_binding,
            "reconciliation_status": "unchanged",
            "reconciliation_reason": "synthetic source-binding control",
        }

    def _source_instance_record(
        self,
        *,
        candidate_id: str | None = None,
        source_instance_id: str | None = None,
        source_binding: dict[str, object],
        participant_refs: list[dict[str, str]] | None = None,
    ) -> dict[str, object]:
        actual_candidate_id = candidate_id or self.CANDIDATE_ID
        return {
            "source_instance_id": source_instance_id or self.SOURCE_INSTANCE_ID,
            "candidate_id": actual_candidate_id,
            "source_binding": source_binding,
            "participant_bindings": [
                {
                    "role": "ordered_participant",
                    "participant_ref": ref,
                }
                for ref in participant_refs
                or [
                    {"participant_kind": "requirement_family", "semantic_ref": "family.a"},
                    {"participant_kind": "requirement_family", "semantic_ref": "family.b"},
                ]
            ],
            "source_context": {
                "zone": "not_applicable",
                "visibility": "not_applicable",
                "timing": "not_applicable",
                "temporal_order": "not_applicable",
                "source_affected_relation": "not_applicable",
                "control_ownership_relation": "not_applicable",
                "replacement_layer_relation": "not_applicable",
                "trigger_lki_relation": "not_applicable",
                "information_relation": "not_applicable",
                "decision_actor_relation": "not_applicable",
            },
        }

    def _synthetic_binding_fixture(
        self,
        *,
        rows: list[list[str]] | None = None,
        candidate_binding_overrides: dict[str, object] | None = None,
        instance_binding_overrides: dict[str, object] | None = None,
        candidate_overrides: dict[str, object] | None = None,
        candidate_identity_override: dict[str, str] | None = None,
        instance_overrides: dict[str, object] | None = None,
        rev3_input_overrides: dict[str, str] | None = None,
        extra_candidates: list[dict[str, object]] | None = None,
        extra_instances: list[dict[str, object]] | None = None,
    ) -> tuple[
        AuthoritySourceResolver,
        SourceBindingDigestV1,
        dict[str, object],
        dict[str, object],
    ]:
        census_rows = rows or [self._source_values()]
        member_raw = self._census_bytes(census_rows)
        member_sha = digest(member_raw)
        archive_raw = archive_bytes({REV3_CENSUS_MEMBER: member_raw})
        archive = Rev3ArchiveStore.from_bytes(archive_raw, digest(archive_raw))

        source_binding: dict[str, object] = {
            "kind": "rev3",
            "archive_member": REV3_CENSUS_MEMBER,
            "archive_member_sha256": member_sha,
            "row_ordinal": 0,
            "source_columns": list(REV3_SOURCE_COLUMNS),
            "source_values": list(census_rows[0]),
        }
        if candidate_binding_overrides:
            source_binding.update(candidate_binding_overrides)
        candidate = self._candidate_record(source_binding=source_binding)
        if candidate_overrides:
            candidate.update(candidate_overrides)
        candidate["candidate_identity"] = deepcopy(
            candidate_identity_override or self._candidate_identity_for_test(candidate)
        )

        instance_binding = deepcopy(source_binding)
        if instance_binding_overrides:
            instance_binding.update(instance_binding_overrides)
        instance = self._source_instance_record(source_binding=instance_binding)
        if instance_overrides:
            instance.update(instance_overrides)

        candidates = [candidate, *(deepcopy(extra_candidates or []))]
        for extra_candidate in candidates[1:]:
            if extra_candidate.get("candidate_identity") == self.CANDIDATE_IDENTITY:
                extra_candidate["candidate_identity"] = self._candidate_identity_for_test(
                    extra_candidate
                )
        instances = [instance, *(extra_instances or [])]
        model_value = {
            "schema": "manafold.m2.5.c.declared-interaction-model.v2",
            "model_id": "declared-interaction-model.v2",
            "participant_kind_vocabulary": ["requirement_family"],
            "participant_role_vocabulary": ["ordered_participant"],
            "context_dimensions": [
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
            ],
            "context_value_vocabulary": {
                key: ["not_applicable"]
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
        model_raw = json_bytes(model_value)
        self.write_repo("sources/m2_5/closures/C/declared_interaction_model.v2.json", model_raw)
        rev3_input = {
            "archive_member": REV3_CENSUS_MEMBER,
            "archive_member_sha256": member_sha,
            "source_package_sha256": digest(archive_raw),
        }
        if rev3_input_overrides:
            rev3_input.update(rev3_input_overrides)
        universe = {
            "schema": CANDIDATE_UNIVERSE_SCHEMA,
            "model_id": "declared-interaction-model.v2",
            "input_bindings": {
                "declared_model": {
                    "path": "sources/m2_5/closures/C/declared_interaction_model.v2.json",
                    "raw_sha256": digest(model_raw),
                },
                "review_additions": {
                    "path": "sources/m2_5/closures/C/interaction_review_additions.v2.json",
                    "raw_sha256": "11" * 32,
                },
                "rev3_candidate_source": rev3_input,
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
            "candidate_count": len(candidates),
            "candidate_reconciliation_counts": {
                "unchanged": len(candidates),
                "stale_rev3_candidate": 0,
                "removed_not_interaction": 0,
                "merged_semantic_duplicate": 0,
                "new_targeted_higher_order_candidate": 0,
                "new_b2_derived": 0,
            },
            "source_instance_count": len(instances),
            "candidates": candidates,
            "source_instances": instances,
        }
        self.write_repo(CANDIDATE_UNIVERSE_PATH, json_bytes(universe))
        binding = SourceBindingDigestV1(
            artifact_role="candidate_universe",
            path=CANDIDATE_UNIVERSE_PATH,
            schema_or_null=CANDIDATE_UNIVERSE_SCHEMA,
            raw_sha256=bytes.fromhex(digest(json_bytes(universe))),
        )
        return (
            AuthoritySourceResolver(self.repo, rev3_archive=archive),
            binding,
            candidate,
            instance,
        )

    def test_exact_candidate_resolution_returns_immutable_record(self) -> None:
        resolver, binding, candidate, _ = self._synthetic_binding_fixture()

        resolved = resolver.resolve_candidate(
            candidate["candidate_id"],
            cast(dict[str, object], candidate["candidate_identity"]),
            binding,
        )

        self.assertEqual(resolved.candidate_id, self.CANDIDATE_ID)
        self.assertEqual(
            resolved.candidate_identity["digest_hex"],
            cast(dict[str, str], candidate["candidate_identity"])["digest_hex"],
        )
        self.assertEqual(resolved.candidate_record["candidate_id"], self.CANDIDATE_ID)
        with self.assertRaises(TypeError):
            cast(dict[str, object], resolved.candidate_record)["candidate_id"] = "mutated"

    def test_exact_source_instance_resolution_verifies_the_rev3_row(self) -> None:
        resolver, binding, candidate, instance = self._synthetic_binding_fixture()
        resolved_candidate = resolver.resolve_candidate(
            candidate["candidate_id"],
            cast(dict[str, object], candidate["candidate_identity"]),
            binding,
        )

        resolved = resolver.resolve_source_instance(
            resolved_candidate, cast(str, instance["source_instance_id"])
        )

        self.assertEqual(resolved.source_instance_id, self.SOURCE_INSTANCE_ID)
        self.assertEqual(
            resolved.source_artifact.raw_bytes.splitlines()[1].split(b",", 1)[0],
            self.CANDIDATE_ID.encode(),
        )
        with self.assertRaises(TypeError):
            cast(dict[str, object], resolved.source_instance_record)["candidate_id"] = "mutated"

    def test_raw_supporting_ids_are_normalized_after_parsing(self) -> None:
        raw_values = self._source_values()
        raw_values[-1] = '["cap.activation_cost", "cap.amass"]'
        resolver, binding, candidate, instance = self._synthetic_binding_fixture(
            rows=[raw_values],
            candidate_overrides={
                "supporting_requirement_ids": ["cap.amass", "cap.activation_cost"]
            },
        )

        resolved = resolver.resolve_candidate_source_instance(
            cast(str, candidate["candidate_id"]),
            cast(dict[str, object], candidate["candidate_identity"]),
            cast(str, instance["source_instance_id"]),
            binding,
        )

        self.assertEqual(
            resolved.candidate.candidate_record["supporting_requirement_ids"],
            ("cap.amass", "cap.activation_cost"),
        )
        self.assertEqual(
            resolved.source_binding["source_values"][-1],
            '["cap.activation_cost", "cap.amass"]',
        )

    def test_rev3_normalization_must_match_candidate_fields(self) -> None:
        resolver, binding, candidate, instance = self._synthetic_binding_fixture()
        universe_path = self.repo / Path(*CANDIDATE_UNIVERSE_PATH.split("/"))
        universe = cast(dict[str, object], json.loads(universe_path.read_text(encoding="utf-8")))
        persisted_candidate = cast(list[dict[str, object]], universe["candidates"])[0]
        persisted_candidate["scope"] = "intra_deck"
        persisted_candidate["candidate_identity"] = self._candidate_identity_for_test(
            persisted_candidate
        )
        raw = json_bytes(universe)
        self.write_repo(CANDIDATE_UNIVERSE_PATH, raw)
        mutated_binding = SourceBindingDigestV1(
            artifact_role="candidate_universe",
            path=CANDIDATE_UNIVERSE_PATH,
            schema_or_null=CANDIDATE_UNIVERSE_SCHEMA,
            raw_sha256=bytes.fromhex(digest(raw)),
        )

        self.assert_resolution_error(
            lambda: resolver.resolve_candidate_source_instance(
                cast(str, candidate["candidate_id"]),
                cast(dict[str, object], persisted_candidate["candidate_identity"]),
                cast(str, instance["source_instance_id"]),
                mutated_binding,
            ),
            ResolutionStatus.FAIL,
            "REV3_CANDIDATE_NORMALIZATION_MISMATCH",
        )

    def test_candidate_and_source_instance_convenience_resolution(self) -> None:
        resolver, binding, candidate, instance = self._synthetic_binding_fixture()

        resolved = resolver.resolve_candidate_source_instance(
            cast(str, candidate["candidate_id"]),
            cast(dict[str, object], candidate["candidate_identity"]),
            cast(str, instance["source_instance_id"]),
            binding,
        )

        self.assertEqual(resolved.candidate.candidate_id, self.CANDIDATE_ID)
        self.assertEqual(resolved.source_instance_id, self.SOURCE_INSTANCE_ID)

    def test_unknown_candidate_fails_closed(self) -> None:
        resolver, binding, candidate, _ = self._synthetic_binding_fixture()
        self.assert_resolution_error(
            lambda: resolver.resolve_candidate(
                "unknown", cast(dict[str, object], candidate["candidate_identity"]), binding
            ),
            ResolutionStatus.FAIL,
            "CANDIDATE_BINDING_MISMATCH",
        )

    def test_candidate_identity_mismatch_fails_closed(self) -> None:
        resolver, binding, candidate, _ = self._synthetic_binding_fixture(
            candidate_identity_override=self.CANDIDATE_IDENTITY
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_candidate(
                cast(str, candidate["candidate_id"]),
                cast(dict[str, object], candidate["candidate_identity"]),
                binding,
            ),
            ResolutionStatus.FAIL,
            "CANDIDATE_IDENTITY_MISMATCH",
        )

    def test_candidate_universe_digest_mismatch_fails_closed(self) -> None:
        resolver, binding, candidate, _ = self._synthetic_binding_fixture()
        wrong_binding = SourceBindingDigestV1(
            artifact_role="candidate_universe",
            path=binding.path,
            schema_or_null=binding.schema_or_null,
            raw_sha256=bytes.fromhex("00" * 32),
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_candidate(
                cast(str, candidate["candidate_id"]),
                cast(dict[str, object], candidate["candidate_identity"]),
                wrong_binding,
            ),
            ResolutionStatus.FAIL,
            "SOURCE_DIGEST_MISMATCH",
        )

    def test_unsupported_candidate_source_kind_fails_closed(self) -> None:
        resolver, binding, candidate, _ = self._synthetic_binding_fixture(
            candidate_overrides={"source_origin": "targeted_higher_order_review"},
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_candidate(
                cast(str, candidate["candidate_id"]),
                cast(dict[str, object], candidate["candidate_identity"]),
                binding,
            ),
            ResolutionStatus.FAIL,
            "SOURCE_KIND_UNSUPPORTED",
        )

    def test_duplicate_candidate_id_fails_closed(self) -> None:
        duplicate = self._candidate_record(
            source_binding={
                "kind": "rev3",
                "archive_member": REV3_CENSUS_MEMBER,
                "archive_member_sha256": "00" * 32,
                "row_ordinal": 0,
                "source_columns": list(REV3_SOURCE_COLUMNS),
                "source_values": self._source_values(),
            }
        )
        resolver, binding, candidate, instance = self._synthetic_binding_fixture(
            extra_candidates=[duplicate],
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_candidate(
                cast(str, candidate["candidate_id"]),
                cast(dict[str, object], candidate["candidate_identity"]),
                binding,
            ),
            ResolutionStatus.FAIL,
            "DUPLICATE_CANDIDATE_ID",
        )

    def test_wrong_source_instance_id_fails_closed(self) -> None:
        resolver, binding, candidate, _ = self._synthetic_binding_fixture()
        resolved_candidate = resolver.resolve_candidate(
            cast(str, candidate["candidate_id"]),
            cast(dict[str, object], candidate["candidate_identity"]),
            binding,
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_source_instance(resolved_candidate, "si.v1/wrong/0"),
            ResolutionStatus.FAIL,
            "SOURCE_INSTANCE_BINDING_MISMATCH",
        )

    def test_wrong_rev3_row_ordinal_fails_closed(self) -> None:
        resolver, binding, candidate, instance = self._synthetic_binding_fixture(
            candidate_binding_overrides={"row_ordinal": 1},
            instance_binding_overrides={"row_ordinal": 1},
        )
        resolved_candidate = resolver.resolve_candidate(
            cast(str, candidate["candidate_id"]),
            cast(dict[str, object], candidate["candidate_identity"]),
            binding,
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_source_instance(
                resolved_candidate, cast(str, instance["source_instance_id"])
            ),
            ResolutionStatus.FAIL,
            "REV3_ROW_ORDINAL_OUT_OF_RANGE",
        )

    def test_in_range_ordinal_pointing_to_wrong_row_fails_closed(self) -> None:
        second_row = self._source_values("CROSS_DECK|P2|family.a|family.b|DIRECTIONAL_BINARY")
        resolver, binding, candidate, instance = self._synthetic_binding_fixture(
            rows=[self._source_values(), second_row],
            candidate_binding_overrides={"row_ordinal": 1},
            instance_binding_overrides={"row_ordinal": 1},
        )
        resolved_candidate = resolver.resolve_candidate(
            cast(str, candidate["candidate_id"]),
            cast(dict[str, object], candidate["candidate_identity"]),
            binding,
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_source_instance(
                resolved_candidate, cast(str, instance["source_instance_id"])
            ),
            ResolutionStatus.FAIL,
            "REV3_SOURCE_ROW_MISMATCH",
        )

    def test_wrong_source_columns_fail_closed(self) -> None:
        resolver, binding, candidate, _ = self._synthetic_binding_fixture(
            instance_binding_overrides={"source_columns": [*REV3_SOURCE_COLUMNS[:-1], "wrong"]},
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_candidate(
                cast(str, candidate["candidate_id"]),
                cast(dict[str, object], candidate["candidate_identity"]),
                binding,
            ),
            ResolutionStatus.FAIL,
            "REV3_SOURCE_COLUMNS_MISMATCH",
        )

    def test_wrong_source_values_fail_closed(self) -> None:
        changed_values = self._source_values()
        changed_values[8] = "tampered source value"
        resolver, binding, candidate, instance = self._synthetic_binding_fixture(
            candidate_binding_overrides={"source_values": changed_values},
            instance_binding_overrides={"source_values": changed_values},
        )
        resolved_candidate = resolver.resolve_candidate(
            cast(str, candidate["candidate_id"]),
            cast(dict[str, object], candidate["candidate_identity"]),
            binding,
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_source_instance(
                resolved_candidate, cast(str, instance["source_instance_id"])
            ),
            ResolutionStatus.FAIL,
            "REV3_SOURCE_ROW_MISMATCH",
        )

    def test_wrong_archive_member_digest_fails_closed(self) -> None:
        resolver, binding, candidate, instance = self._synthetic_binding_fixture(
            candidate_binding_overrides={"archive_member_sha256": "00" * 32},
            instance_binding_overrides={"archive_member_sha256": "00" * 32},
            rev3_input_overrides={"archive_member_sha256": "00" * 32},
        )
        resolved_candidate = resolver.resolve_candidate(
            cast(str, candidate["candidate_id"]),
            cast(dict[str, object], candidate["candidate_identity"]),
            binding,
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_source_instance(
                resolved_candidate, cast(str, instance["source_instance_id"])
            ),
            ResolutionStatus.FAIL,
            "SOURCE_DIGEST_MISMATCH",
        )

    def test_wrong_archive_member_fails_closed(self) -> None:
        resolver, binding, candidate, _ = self._synthetic_binding_fixture(
            candidate_binding_overrides={"archive_member": "derived/wrong.csv"},
            instance_binding_overrides={"archive_member": "derived/wrong.csv"},
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_candidate(
                cast(str, candidate["candidate_id"]),
                cast(dict[str, object], candidate["candidate_identity"]),
                binding,
            ),
            ResolutionStatus.FAIL,
            "SOURCE_MEMBER_MISMATCH",
        )

    def test_candidate_source_instance_cross_binding_mismatch_fails_closed(self) -> None:
        resolver, binding, candidate, _ = self._synthetic_binding_fixture(
            instance_overrides={"candidate_id": "CROSS_DECK|P9|foreign|foreign|DIRECTIONAL_BINARY"},
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_candidate(
                cast(str, candidate["candidate_id"]),
                cast(dict[str, object], candidate["candidate_identity"]),
                binding,
            ),
            ResolutionStatus.FAIL,
            "SOURCE_INSTANCE_CANDIDATE_MISMATCH",
        )

    def test_missing_external_rev3_archive_is_blocked(self) -> None:
        resolver, binding, candidate, instance = self._synthetic_binding_fixture()
        resolver = AuthoritySourceResolver(self.repo, rev3_archive_root=self.repo / "missing")
        resolved_candidate = resolver.resolve_candidate(
            cast(str, candidate["candidate_id"]),
            cast(dict[str, object], candidate["candidate_identity"]),
            binding,
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_source_instance(
                resolved_candidate, cast(str, instance["source_instance_id"])
            ),
            ResolutionStatus.BLOCKED,
            "REV3_ARCHIVE_SOURCE_UNAVAILABLE",
        )

    def test_missing_repository_candidate_ledger_is_fail(self) -> None:
        resolver = AuthoritySourceResolver(self.repo)
        binding = SourceBindingDigestV1(
            artifact_role="candidate_universe",
            path=CANDIDATE_UNIVERSE_PATH,
            schema_or_null=CANDIDATE_UNIVERSE_SCHEMA,
            raw_sha256=bytes(32),
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_candidate("candidate", self.CANDIDATE_IDENTITY, binding),
            ResolutionStatus.FAIL,
            "REPOSITORY_SOURCE_MISSING",
        )

    def test_real_candidate_ledger_probe_resolves_candidate_before_external_rev3(self) -> None:
        source = ROOT / "sources/m2_5/closures/C/interaction_candidate_universe.v2.json"
        raw = source.read_bytes()
        document = cast(dict[str, object], json.loads(raw.decode("utf-8")))
        candidates = cast(list[dict[str, object]], document["candidates"])
        instances = cast(list[dict[str, object]], document["source_instances"])
        first = candidates[0]
        first_instance = instances[0]
        resolver = AuthoritySourceResolver(ROOT, rev3_archive_root=ROOT / "missing")
        binding = SourceBindingDigestV1(
            artifact_role="candidate_universe",
            path=CANDIDATE_UNIVERSE_PATH,
            schema_or_null=CANDIDATE_UNIVERSE_SCHEMA,
            raw_sha256=bytes.fromhex(digest(raw)),
        )

        resolved = resolver.resolve_candidate(
            cast(str, first["candidate_id"]),
            cast(dict[str, object], first["candidate_identity"]),
            binding,
        )

        self.assertEqual(resolved.candidate_record["candidate_id"], first["candidate_id"])
        self.assertEqual(resolved.candidate_universe.raw_sha256, digest(raw))
        self.assert_resolution_error(
            lambda: resolver.resolve_source_instance(
                resolved, cast(str, first_instance["source_instance_id"])
            ),
            ResolutionStatus.BLOCKED,
            "REV3_ARCHIVE_SOURCE_UNAVAILABLE",
        )

    def test_repository_verifies_digest_before_json_parsing(self) -> None:
        resolver = AuthoritySourceResolver(self.repo)
        raw = b"{not valid JSON"
        self.write_repo("artifact.json", raw)

        self.assert_resolution_error(
            lambda: resolver.resolve_repository_artifact(
                "artifact.json", "00" * 32, "example.schema.v1"
            ),
            ResolutionStatus.FAIL,
            "SOURCE_DIGEST_MISMATCH",
        )

        valid = json_bytes({"schema": "example.schema.v1", "value": 7})
        self.write_repo("artifact.json", valid)
        resolved = resolver.resolve_repository_artifact(
            "artifact.json", digest(valid), "example.schema.v1"
        )
        self.assertEqual(resolved.raw_bytes, valid)
        self.assertEqual(resolved.raw_sha256, digest(valid))
        self.assertEqual(resolved.json_value, {"schema": "example.schema.v1", "value": 7})

    def test_missing_repository_source_is_fail_not_blocked(self) -> None:
        resolver = AuthoritySourceResolver(self.repo)
        self.assert_resolution_error(
            lambda: resolver.resolve_repository_artifact(
                "missing.json", "00" * 32, "example.schema.v1"
            ),
            ResolutionStatus.FAIL,
            "REPOSITORY_SOURCE_MISSING",
        )

    def test_repository_paths_are_confined_and_normalized(self) -> None:
        resolver = AuthoritySourceResolver(self.repo)
        for relative in ("../outside.json", "/absolute.json", "C:/outside.json", "a\\b.json"):
            with self.subTest(relative=relative):

                def resolve_invalid_path(relative: str = relative) -> object:
                    return resolver.resolve_repository_artifact(relative, "00" * 32, None)

                self.assert_resolution_error(
                    resolve_invalid_path,
                    ResolutionStatus.FAIL,
                    "PATH_INVALID",
                )

    def test_json_pointer_uses_strict_rfc6901_evaluation(self) -> None:
        value = {
            "schema": "example.schema.v1",
            "a/b": {"m~n": ["selected"]},
            "array": ["first", "second"],
        }
        raw = json_bytes(value)
        self.write_repo("artifact.json", raw)
        resolver = AuthoritySourceResolver(self.repo)
        artifact = resolver.resolve_repository_artifact(
            "artifact.json", digest(raw), "example.schema.v1"
        )

        self.assertEqual(
            resolver.resolve_locator(artifact, ("json_pointer", "")).value,
            value,
        )
        self.assertEqual(
            resolver.resolve_locator(artifact, ("json_pointer", "/a~1b/m~0n/0")).value,
            "selected",
        )
        self.assertEqual(
            resolver.resolve_locator(artifact, ("json_pointer", "/array/1")).value,
            "second",
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_locator(artifact, ("json_pointer", "/a~2b")),
            ResolutionStatus.FAIL,
            "LOCATOR_INVALID",
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_locator(artifact, ("json_pointer", "/array/01")),
            ResolutionStatus.FAIL,
            "LOCATOR_UNRESOLVED",
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_locator(artifact, ("json_pointer", "/array/" + "\u0661")),
            ResolutionStatus.FAIL,
            "LOCATOR_UNRESOLVED",
        )

    def test_locator_revalidates_raw_bytes_and_rejects_unverified_artifacts(self) -> None:
        value = {"schema": "example.schema.v1", "nested": {"value": "original"}}
        raw = json_bytes(value)
        self.write_repo("artifact.json", raw)
        resolver = AuthoritySourceResolver(self.repo)
        artifact = resolver.resolve_repository_artifact(
            "artifact.json", digest(raw), "example.schema.v1"
        )
        cast(dict[str, object], cast(dict[str, object], artifact.json_value)["nested"])["value"] = (
            "mutated"
        )

        self.assertEqual(
            resolver.resolve_locator(artifact, ("json_pointer", "/nested/value")).value,
            "original",
        )
        self.assertEqual(
            resolver.resolve_locator(artifact, ("whole_artifact", None)).value,
            value,
        )

        forged = ResolvedArtifact(
            source_kind="repository",
            path="artifact.json",
            raw_bytes=raw,
            raw_sha256=digest(raw),
            schema_or_null="example.schema.v1",
            json_value=value,
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_locator(forged, ("whole_artifact", None)),
            ResolutionStatus.FAIL,
            "ARTIFACT_UNVERIFIED",
        )

    def test_archive_reads_only_manifest_verified_members(self) -> None:
        member = b"candidate,relation\nrow-0,review\n"
        raw = archive_bytes({"derived/example.csv": member})
        archive = Rev3ArchiveStore.from_bytes(raw, digest(raw))
        resolver = AuthoritySourceResolver(self.repo, rev3_archive=archive)

        resolved = resolver.resolve_rev3_locator(
            ("archive_member", "derived/example.csv"), digest(member)
        )
        self.assertEqual(resolved.value, member)
        self.assertEqual(resolved.artifact.path, "derived/example.csv")

    def test_archive_failures_are_fail_not_blocked(self) -> None:
        member = b"member"
        valid = archive_bytes({"derived/example.csv": member})
        self.assert_resolution_error(
            lambda: Rev3ArchiveStore.from_bytes(valid, "00" * 32),
            ResolutionStatus.FAIL,
            "REV3_ARCHIVE_DIGEST_MISMATCH",
        )
        self.assert_resolution_error(
            lambda: Rev3ArchiveStore.from_bytes(b"not a zip", digest(b"not a zip")),
            ResolutionStatus.FAIL,
            "REV3_ARCHIVE_INVALID",
        )

        missing = archive_bytes(
            {"derived/example.csv": member},
            manifest_entries=[{"path": "derived/missing.csv", "bytes": 1, "sha256": digest(b"x")}],
        )
        self.assert_resolution_error(
            lambda: Rev3ArchiveStore.from_bytes(missing, digest(missing)),
            ResolutionStatus.FAIL,
            "REV3_MEMBER_MISSING",
        )

        wrong_digest = archive_bytes(
            {"derived/example.csv": member},
            manifest_entries=[
                {"path": "derived/example.csv", "bytes": len(member), "sha256": "00" * 32}
            ],
        )
        self.assert_resolution_error(
            lambda: Rev3ArchiveStore.from_bytes(wrong_digest, digest(wrong_digest)),
            ResolutionStatus.FAIL,
            "REV3_MEMBER_DIGEST_MISMATCH",
        )

        duplicate = archive_bytes(
            {"derived/example.csv": member}, duplicate_member="derived/example.csv"
        )
        self.assert_resolution_error(
            lambda: Rev3ArchiveStore.from_bytes(duplicate, digest(duplicate)),
            ResolutionStatus.FAIL,
            "REV3_MEMBER_DUPLICATE",
        )

    def test_missing_configured_rev3_archive_is_blocked(self) -> None:
        resolver = AuthoritySourceResolver(self.repo, rev3_archive_root=self.repo / "missing")
        binding = SourceBindingDigestV1(
            artifact_role="rev3_source",
            path=REV3_CENSUS_MEMBER,
            schema_or_null=None,
            raw_sha256=bytes.fromhex(
                "82f9312113bb1007ad6562d454c515f85dbc1e0d7a471f7b1c6793725aea45d4"
            ),
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_source_binding(binding),
            ResolutionStatus.BLOCKED,
            "REV3_ARCHIVE_SOURCE_UNAVAILABLE",
        )

    def test_real_static_model_probe_resolves_after_bytes_and_schema_check(self) -> None:
        source_root = ROOT / "sources/m2_5/closures/C"
        raw = (source_root / "declared_interaction_model.v2.json").read_bytes()
        resolver = AuthoritySourceResolver(ROOT)
        binding = SourceBindingDigestV1(
            artifact_role="declared_model",
            path="sources/m2_5/closures/C/declared_interaction_model.v2.json",
            schema_or_null="manafold.m2.5.c.declared-interaction-model.v2",
            raw_sha256=bytes.fromhex(digest(raw)),
        )
        resolved = resolver.resolve_source_binding(binding)
        self.assertEqual(resolved.raw_bytes, raw)
        resolved_json = cast(dict[str, object], resolved.json_value)
        self.assertEqual(resolved_json["schema"], "manafold.m2.5.c.declared-interaction-model.v2")

    def test_content_addressed_acceptance_event_and_roster_leaves_are_verified(self) -> None:
        resolver = AuthoritySourceResolver(self.repo)
        event_path = ROOT / "conformance/fixtures/authority/review_acceptance_event.v1.json"
        event_raw = event_path.read_bytes()
        event = json.loads(event_raw)
        event_id = event["event_id"]
        event_relative = (
            "sources/m2_5/authorities/review_acceptance_events/v1/"
            + event_id.split("/", maxsplit=1)[1]
            + ".json"
        )
        self.write_repo(event_relative, event_raw)
        event_ref = ReviewEventRefV1(
            path=event_relative,
            raw_sha256=bytes.fromhex(digest(event_raw)),
            event_id=event_id,
        )
        resolved_event = resolver.resolve_acceptance_event_leaf(event_ref)
        resolved_event_json = cast(dict[str, object], resolved_event.json_value)
        self.assertEqual(resolved_event_json["event_id"], event_id)
        self.assertEqual(resolved_event.schema_or_null, ACCEPTANCE_EVENT_SCHEMA_V1)
        self.assertEqual(
            resolver.resolve_locator(resolved_event, ("event_id", event_id)).value,
            event,
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_locator(resolved_event, ("event_id", "ae.v1/" + "11" * 32)),
            ResolutionStatus.FAIL,
            "ACCEPTANCE_EVENT_ID_MISMATCH",
        )
        for field, value in (("decision", "not_accepted"), ("checklist_id", "other.v1")):
            with self.subTest(field=field):
                changed = dict(event)
                changed[field] = value
                changed_raw = json_bytes(changed)
                self.write_repo(event_relative, changed_raw)
                changed_ref = ReviewEventRefV1(
                    path=event_relative,
                    raw_sha256=bytes.fromhex(digest(changed_raw)),
                    event_id=event_id,
                )

                def resolve_changed_event(reference: ReviewEventRefV1 = changed_ref) -> object:
                    return resolver.resolve_acceptance_event_leaf(reference)

                self.assert_resolution_error(
                    resolve_changed_event,
                    ResolutionStatus.FAIL,
                    "ACCEPTANCE_EVENT_INVALID",
                )

        roster_path = ROOT / "conformance/fixtures/authority/reviewer_roster.v1.json"
        roster_raw = roster_path.read_bytes()
        roster_digest = bytes.fromhex(digest(roster_raw))
        roster_relative = (
            "sources/m2_5/authorities/reviewer_rosters/v1/" + roster_digest.hex() + ".json"
        )
        self.write_repo(roster_relative, roster_raw)
        roster_ref = ReviewerRosterRefV1(
            path=roster_relative,
            schema="manafold.m2.5.c.reviewer-roster.v1",
            raw_sha256=roster_digest,
        )
        resolved_roster = resolver.resolve_reviewer_roster_leaf(roster_ref)
        self.assertEqual(resolved_roster.json_value["schema"], roster_ref.schema)

        for field, nested in (
            ("reviewer_role_bindings", {"extra": True}),
            ("source_binding_digests", {"extra": True}),
            ("review_evidence_refs", {"extra": True}),
        ):
            with self.subTest(field=field):
                changed = dict(event)
                changed_items = [dict(item, **nested) for item in event[field]]
                changed[field] = changed_items
                changed_raw = json_bytes(changed)
                self.write_repo(event_relative, changed_raw)
                changed_ref = ReviewEventRefV1(
                    path=event_relative,
                    raw_sha256=bytes.fromhex(digest(changed_raw)),
                    event_id=event_id,
                )
                self.assert_resolution_error(
                    lambda changed_ref=changed_ref: resolver.resolve_acceptance_event_leaf(
                        changed_ref
                    ),
                    ResolutionStatus.FAIL,
                    "ACCEPTANCE_EVENT_INVALID",
                )

        changed = dict(event)
        changed["review_evidence_refs"] = [
            dict(
                event["review_evidence_refs"][0], locator={"kind": "whole_artifact", "value": None}
            )
        ]
        changed_raw = json_bytes(changed)
        self.write_repo(event_relative, changed_raw)
        changed_ref = ReviewEventRefV1(
            path=event_relative,
            raw_sha256=bytes.fromhex(digest(changed_raw)),
            event_id=event_id,
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_acceptance_event_leaf(changed_ref),
            ResolutionStatus.FAIL,
            "ACCEPTANCE_EVENT_INVALID",
        )

        tampered = dict(event)
        tampered["event_id"] = "ae.v1/" + "11" * 32
        tampered_raw = json_bytes(tampered)
        self.write_repo(event_relative, tampered_raw)
        tampered_ref = ReviewEventRefV1(
            path=event_relative,
            raw_sha256=bytes.fromhex(digest(tampered_raw)),
            event_id=event_id,
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_acceptance_event_leaf(tampered_ref),
            ResolutionStatus.FAIL,
            "ACCEPTANCE_EVENT_ID_MISMATCH",
        )


class B2SourceResolverTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.repo = Path(self.tempdir.name) / "repo"
        self.repo.mkdir()
        source = ROOT / "sources/m2_5/closures/B2"
        target = self.repo / "sources/m2_5/closures/B2"
        shutil.copytree(source, target)

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def assert_resolution_error(
        self,
        operation: Callable[[], object],
        status: ResolutionStatus,
        code: str,
    ) -> None:
        with self.assertRaises(ResolutionError) as context:
            operation()
        self.assertEqual(context.exception.status, status)
        self.assertEqual(context.exception.code, code)

    def _binding(
        self, role: str, path: str, schema: str, root: Path | None = None
    ) -> SourceBindingDigestV1:
        source_root = root or self.repo
        raw = (source_root / Path(*path.split("/"))).read_bytes()
        return SourceBindingDigestV1(
            artifact_role=role,
            path=path,
            schema_or_null=schema,
            raw_sha256=bytes.fromhex(digest(raw)),
        )

    def _bindings(self, root: Path | None = None) -> B2ArtifactBindingsV1:
        return B2ArtifactBindingsV1(
            catalog=self._binding("b2_catalog", B2_CATALOG_PATH, B2_CATALOG_SCHEMA, root),
            classifications=self._binding(
                "b2_classifications", B2_CLASSIFICATION_PATH, B2_CLASSIFICATION_SCHEMA, root
            ),
            closure=self._binding("b2_closure", B2_CLOSURE_PATH, B2_CLOSURE_SCHEMA, root),
        )

    def _rewrite_json(self, path: str, mutate: Callable[[dict[str, object]], None]) -> None:
        file_path = self.repo / Path(*path.split("/"))
        document = cast(dict[str, object], json.loads(file_path.read_text(encoding="utf-8")))
        mutate(document)
        file_path.write_bytes(json_bytes(document))

    def _rebind_closure(self, artifact_name: str) -> None:
        closure_path = self.repo / Path(*B2_CLOSURE_PATH.split("/"))
        closure = cast(dict[str, object], json.loads(closure_path.read_text(encoding="utf-8")))
        artifact_path = self.repo / Path("sources/m2_5/closures/B2", *artifact_name.split("/"))
        artifact_digest = digest(artifact_path.read_bytes())
        for item in cast(list[dict[str, object]], closure["bound_artifacts"]):
            if item["path"] == artifact_name:
                item["raw_sha256"] = artifact_digest
        closure_path.write_bytes(json_bytes(closure))

    def _real_first_records(self, root: Path | None = None) -> tuple[str, dict[str, object], str]:
        source_root = root or self.repo
        document = cast(
            dict[str, object],
            json.loads(
                (source_root / Path(*B2_CLASSIFICATION_PATH.split("/"))).read_text(encoding="utf-8")
            ),
        )
        classification = cast(list[dict[str, object]], document["classifications"])[0]
        family_id = cast(
            str,
            cast(list[dict[str, object]], classification["requirement_assignments"])[0][
                "requirement_family_id"
            ],
        )
        return cast(str, classification["oracle_semantic_identity"]), classification, family_id

    def test_real_b2_family_classification_assignment_and_boundary_resolve(self) -> None:
        resolver = AuthoritySourceResolver(ROOT)
        bindings = self._bindings(ROOT)
        classification_document = json.loads(
            (ROOT / Path(*B2_CLASSIFICATION_PATH.split("/"))).read_text(encoding="utf-8")
        )
        classification = classification_document["classifications"][0]
        family_id = classification["requirement_assignments"][0]["requirement_family_id"]

        family = resolver.resolve_b2_requirement_family(family_id, bindings)
        resolved_classification = resolver.resolve_b2_classification(
            classification["oracle_semantic_identity"],
            classification["classification_identity"],
            bindings,
        )
        assignment = resolver.resolve_b2_assignment(
            resolved_classification,
            family_id,
            bindings,
        )
        boundary = resolver.resolve_b2_boundary(
            family,
            B2FamilyRefV1(
                family_id=family_id,
                lifecycle="active",
                assignment_role="primary",
            ),
            cast(str, family.record["precise_semantic_definition"]),
            assignment,
        )

        self.assertEqual(family.family_id, family_id)
        self.assertEqual(
            resolved_classification.oracle_semantic_identity,
            classification["oracle_semantic_identity"],
        )
        self.assertEqual(assignment.assignment["requirement_family_id"], family_id)
        self.assertEqual(boundary.family_ref.family_id, family_id)
        with self.assertRaises(TypeError):
            cast(dict[str, object], family.record)["family_id"] = "mutated"
        with self.assertRaises(TypeError):
            cast(dict[str, object], family.artifact.json_value)["families"] = ()

    def test_checked_in_b2_artifacts_are_read_only_probe_inputs(self) -> None:
        resolver = AuthoritySourceResolver(ROOT)
        bindings = self._bindings(ROOT)
        osi, classification, family_id = self._real_first_records(ROOT)

        family = resolver.resolve_b2_requirement_family(family_id, bindings)
        resolved_classification = resolver.resolve_b2_classification(
            osi, classification["classification_identity"], bindings
        )
        assignment = resolver.resolve_b2_assignment(resolved_classification, family_id, bindings)
        resolved = resolver.resolve_b2_boundary(
            family,
            B2FamilyRefV1(family_id=family_id, lifecycle="active", assignment_role="primary"),
            cast(str, family.record["precise_semantic_definition"]),
            assignment,
        )

        self.assertEqual(resolved.family.artifact.path, B2_CATALOG_PATH)
        self.assertEqual(resolved.assignment.classification.artifact.path, B2_CLASSIFICATION_PATH)

    def test_unknown_b2_family_fails_closed(self) -> None:
        resolver = AuthoritySourceResolver(ROOT)
        self.assertRaises(
            ResolutionError,
            resolver.resolve_b2_requirement_family,
            "cap.not_present",
            self._bindings(),
        )

    def test_wrong_b2_classification_identity_fails_closed(self) -> None:
        resolver = AuthoritySourceResolver(self.repo)
        osi, classification, _ = self._real_first_records()
        identity = deepcopy(cast(dict[str, object], classification["classification_identity"]))
        identity["digest_hex"] = "00" * 32
        self.assert_resolution_error(
            lambda: resolver.resolve_b2_classification(osi, identity, self._bindings()),
            ResolutionStatus.FAIL,
            "B2_CLASSIFICATION_IDENTITY_MISMATCH",
        )

    def test_missing_b2_artifact_is_repository_fail(self) -> None:
        bindings = self._bindings()
        (self.repo / Path(*B2_CATALOG_PATH.split("/"))).unlink()
        resolver = AuthoritySourceResolver(self.repo)
        self.assert_resolution_error(
            lambda: resolver.resolve_b2_requirement_family("cap.aura", bindings),
            ResolutionStatus.FAIL,
            "REPOSITORY_SOURCE_MISSING",
        )

    def test_b2_catalog_duplicate_family_is_ambiguous(self) -> None:
        def duplicate(document: dict[str, object]) -> None:
            families = cast(list[object], document["families"])
            families.append(deepcopy(families[0]))
            document["catalog_family_count"] = 217

        self._rewrite_json(B2_CATALOG_PATH, duplicate)
        self._rebind_closure("requirement_family_catalog.v1.json")
        resolver = AuthoritySourceResolver(self.repo)
        self.assert_resolution_error(
            lambda: resolver.resolve_b2_requirement_family("cap.aura", self._bindings()),
            ResolutionStatus.FAIL,
            "B2_FAMILY_AMBIGUOUS",
        )

    def test_b2_classification_duplicate_osi_is_ambiguous(self) -> None:
        def duplicate(document: dict[str, object]) -> None:
            records = cast(list[object], document["classifications"])
            records[-1] = deepcopy(records[0])

        self._rewrite_json(B2_CLASSIFICATION_PATH, duplicate)
        self._rebind_closure("card_semantic_classifications.v1.json")
        resolver = AuthoritySourceResolver(self.repo)
        osi, classification, _ = self._real_first_records()
        self.assert_resolution_error(
            lambda: resolver.resolve_b2_classification(
                osi, classification["classification_identity"], self._bindings()
            ),
            ResolutionStatus.FAIL,
            "B2_CLASSIFICATION_AMBIGUOUS",
        )

    def test_b2_assignment_and_boundary_cross_binding_fail_closed(self) -> None:
        resolver = AuthoritySourceResolver(self.repo)
        bindings = self._bindings()
        osi, classification, family_id = self._real_first_records()
        resolved_classification = resolver.resolve_b2_classification(
            osi, classification["classification_identity"], bindings
        )
        assignment = resolver.resolve_b2_assignment(resolved_classification, family_id, bindings)
        family = resolver.resolve_b2_requirement_family(family_id, bindings)
        self.assert_resolution_error(
            lambda: resolver.resolve_b2_assignment(
                resolved_classification, "cap.not_present", bindings
            ),
            ResolutionStatus.FAIL,
            "B2_ASSIGNMENT_NOT_FOUND",
        )
        wrong_definition = cast(str, family.record["precise_semantic_definition"]) + "|tampered"
        self.assert_resolution_error(
            lambda: resolver.resolve_b2_boundary(
                family,
                B2FamilyRefV1(family_id=family_id, lifecycle="active", assignment_role="primary"),
                wrong_definition,
                assignment,
            ),
            ResolutionStatus.FAIL,
            "B2_BOUNDARY_BINDING_MISMATCH",
        )
        other_family_id = "cap.copy"
        other_family = resolver.resolve_b2_requirement_family(other_family_id, bindings)
        self.assert_resolution_error(
            lambda: resolver.resolve_b2_boundary(
                other_family,
                B2FamilyRefV1(
                    family_id=other_family_id, lifecycle="active", assignment_role="primary"
                ),
                cast(str, other_family.record["precise_semantic_definition"]),
                assignment,
            ),
            ResolutionStatus.FAIL,
            "B2_BOUNDARY_BINDING_MISMATCH",
        )

    def test_b2_closure_digest_binding_is_verified(self) -> None:
        closure_path = self.repo / Path(*B2_CLOSURE_PATH.split("/"))
        closure = cast(dict[str, object], json.loads(closure_path.read_text(encoding="utf-8")))
        for item in cast(list[dict[str, object]], closure["bound_artifacts"]):
            if item["path"] == "requirement_family_catalog.v1.json":
                item["raw_sha256"] = "00" * 32
        closure_path.write_bytes(json_bytes(closure))
        resolver = AuthoritySourceResolver(self.repo)
        self.assert_resolution_error(
            lambda: resolver.resolve_b2_requirement_family("cap.aura", self._bindings()),
            ResolutionStatus.FAIL,
            "SOURCE_DIGEST_MISMATCH",
        )

    def test_b2_unsupported_family_shape_fails_closed(self) -> None:
        def add_field(document: dict[str, object]) -> None:
            family = cast(list[dict[str, object]], document["families"])[0]
            family["unsupported"] = True

        self._rewrite_json(B2_CATALOG_PATH, add_field)
        self._rebind_closure("requirement_family_catalog.v1.json")
        resolver = AuthoritySourceResolver(self.repo)
        self.assert_resolution_error(
            lambda: resolver.resolve_b2_requirement_family("cap.aura", self._bindings()),
            ResolutionStatus.FAIL,
            "SCHEMA_MISMATCH",
        )


if __name__ == "__main__":
    unittest.main()
