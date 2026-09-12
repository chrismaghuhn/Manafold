from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from shutil import copytree

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "src"))

from authority_source_resolver import ResolutionError
from b2_closure_downstream_readiness import (
    B2_DOWNSTREAM_OWNERSHIP,
    B2DownstreamReadinessConsumer,
    B2DownstreamReadinessError,
    B2DownstreamReadinessStatus,
    prepare_b2_downstream_readiness,
    validate_b2_downstream_readiness,
)
from b2_closure_source_resolver import (
    B2ClosureResolutionMode,
    resolve_b2_closure,
)
from mtgml.authority import (
    CONTEXT_AUTHORITY_SOURCE_ROLES_V2,
    CONTEXT_AUTHORITY_SOURCE_ROLES_V3,
    RELATION_AUTHORITY_SOURCE_ROLES_V2,
    ReviewAuthorityArtifactRoleV4,
)
from mtgml.b2_closure_contract import B2_CLOSURE_V2_PATH
from mtgml.host_binding import V2_SOURCE_ROLES


class B2ClosureDownstreamReadinessTests(unittest.TestCase):
    def test_slice4_readiness_module_exists(self) -> None:
        self.assertIsNotNone(importlib.util.find_spec("b2_closure_downstream_readiness"))

    def test_ownership_matrix_does_not_claim_unimplemented_consumers_ready(self) -> None:
        required = {
            B2DownstreamReadinessConsumer.AUTHORITY_SOURCE_RESOLUTION,
            B2DownstreamReadinessConsumer.AUTHORITY_VALIDATOR,
            B2DownstreamReadinessConsumer.B1_CURRENT_EVIDENCE_ROOT,
            B2DownstreamReadinessConsumer.C_CURRENT_SOURCE_ROOT,
            B2DownstreamReadinessConsumer.RELATION_APPLICATION,
        }
        for consumer in B2DownstreamReadinessConsumer:
            with self.subTest(consumer=consumer):
                self.assertEqual(
                    B2_DOWNSTREAM_OWNERSHIP[consumer].status,
                    B2DownstreamReadinessStatus.ALREADY_READY
                    if consumer in required
                    else B2DownstreamReadinessStatus.NOT_APPLICABLE,
                )

    def test_applicable_source_resolver_is_ready_without_becoming_current(self) -> None:
        readiness = prepare_b2_downstream_readiness(
            ROOT, B2DownstreamReadinessConsumer.AUTHORITY_SOURCE_RESOLUTION
        )
        self.assertTrue(readiness.v2_ready)
        self.assertFalse(readiness.v2_current)
        self.assertEqual(readiness.resolution.mode, B2ClosureResolutionMode.CANDIDATE_V2)
        self.assertTrue(readiness.resolution.v2_verified)
        self.assertFalse(readiness.production_record_created)

    def test_required_owner_paths_consume_the_explicit_v2_witness(self) -> None:
        required = (
            B2DownstreamReadinessConsumer.AUTHORITY_SOURCE_RESOLUTION,
            B2DownstreamReadinessConsumer.AUTHORITY_VALIDATOR,
            B2DownstreamReadinessConsumer.B1_CURRENT_EVIDENCE_ROOT,
            B2DownstreamReadinessConsumer.C_CURRENT_SOURCE_ROOT,
            B2DownstreamReadinessConsumer.RELATION_APPLICATION,
        )
        for consumer in required:
            with self.subTest(consumer=consumer):
                readiness = prepare_b2_downstream_readiness(ROOT, consumer)
                self.assertTrue(readiness.v2_ready)
                self.assertFalse(readiness.v2_current)

    def test_not_applicable_consumers_fail_closed_instead_of_claiming_readiness(self) -> None:
        for consumer in B2DownstreamReadinessConsumer:
            if (
                B2_DOWNSTREAM_OWNERSHIP[consumer].status
                is B2DownstreamReadinessStatus.ALREADY_READY
            ):
                continue
            with (
                self.subTest(consumer=consumer),
                self.assertRaises(B2DownstreamReadinessError) as failure,
            ):
                prepare_b2_downstream_readiness(ROOT, consumer)
            self.assertEqual(failure.exception.code, "DOWNSTREAM_CONSUMER_NOT_APPLICABLE")

    def test_readiness_is_deterministic_and_has_no_current_root_side_effect(self) -> None:
        current_root = ROOT / "sources/m2_5/closures/B2/current_root.json"
        current_root_before = current_root.read_bytes()
        first = prepare_b2_downstream_readiness(
            ROOT, B2DownstreamReadinessConsumer.AUTHORITY_SOURCE_RESOLUTION
        )
        second = prepare_b2_downstream_readiness(
            ROOT, B2DownstreamReadinessConsumer.AUTHORITY_SOURCE_RESOLUTION
        )
        self.assertEqual(first, second)
        self.assertEqual(current_root.read_bytes(), current_root_before)

    def test_historical_v1_resolution_cannot_satisfy_v2_readiness(self) -> None:
        v1 = resolve_b2_closure(ROOT, B2ClosureResolutionMode.HISTORICAL_V1)
        readiness = replace(
            prepare_b2_downstream_readiness(
                ROOT, B2DownstreamReadinessConsumer.AUTHORITY_SOURCE_RESOLUTION
            ),
            resolution=v1,
        )
        with self.assertRaises(B2DownstreamReadinessError) as failure:
            validate_b2_downstream_readiness(ROOT, readiness)
        self.assertEqual(failure.exception.code, "CURRENT_CONSUMER_VERSION_UNSUPPORTED")

    def test_unknown_consumer_fails_closed(self) -> None:
        with self.assertRaises(B2DownstreamReadinessError) as failure:
            prepare_b2_downstream_readiness(ROOT, "unknown_consumer")
        self.assertEqual(failure.exception.code, "UNKNOWN_DOWNSTREAM_CONSUMER")

    def test_tampered_v2_bytes_fail_without_legacy_fallback(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            copytree(ROOT / "sources", repo / "sources")
            copytree(ROOT / "schemas", repo / "schemas")
            path = repo / Path(*B2_CLOSURE_V2_PATH.split("/"))
            path.write_bytes(path.read_bytes() + b"\n")
            with self.assertRaises(ResolutionError) as failure:
                prepare_b2_downstream_readiness(
                    repo, B2DownstreamReadinessConsumer.AUTHORITY_SOURCE_RESOLUTION
                )
        self.assertEqual(failure.exception.code, "SOURCE_DIGEST_MISMATCH")

    def test_forged_verified_witness_is_revalidated_against_repository(self) -> None:
        valid = prepare_b2_downstream_readiness(
            ROOT, B2DownstreamReadinessConsumer.AUTHORITY_SOURCE_RESOLUTION
        )
        forged_resolution = replace(
            valid.resolution,
            binding=replace(
                valid.resolution.binding,
                repository_relative_path="sources/m2_5/closures/B2/wrong.json",
            ),
        )
        forged = replace(valid, resolution=forged_resolution)
        with self.assertRaises(ResolutionError) as failure:
            validate_b2_downstream_readiness(ROOT, forged)
        self.assertEqual(failure.exception.code, "CURRENT_ROOT_PATH_MISMATCH")

    def test_forged_current_flag_is_rejected(self) -> None:
        valid = prepare_b2_downstream_readiness(
            ROOT, B2DownstreamReadinessConsumer.AUTHORITY_SOURCE_RESOLUTION
        )
        forged = replace(valid, resolution=replace(valid.resolution, v2_current=True))
        with self.assertRaises(B2DownstreamReadinessError) as failure:
            validate_b2_downstream_readiness(ROOT, forged)
        self.assertEqual(failure.exception.code, "CURRENT_ROOT_ADOPTION_FORBIDDEN")

    def test_historical_role_registries_are_not_widened(self) -> None:
        self.assertNotIn("b2_closure_v2", {role.value for role in ReviewAuthorityArtifactRoleV4})
        self.assertNotIn("b2_closure_v2", CONTEXT_AUTHORITY_SOURCE_ROLES_V2)
        self.assertNotIn("b2_closure_v2", CONTEXT_AUTHORITY_SOURCE_ROLES_V3)
        self.assertNotIn("b2_closure_v2", RELATION_AUTHORITY_SOURCE_ROLES_V2)
        self.assertNotIn("b2_closure_v2", V2_SOURCE_ROLES)

    def test_readiness_does_not_rewrite_historical_binding_bytes(self) -> None:
        paths = (
            ROOT / "sources/m2_5/closures/B2/classification_closure.v1.json",
            ROOT / "sources/m2_5/closures/B2/classification_closure.v2.json",
        )
        before = tuple(path.read_bytes() for path in paths)
        prepare_b2_downstream_readiness(
            ROOT, B2DownstreamReadinessConsumer.AUTHORITY_SOURCE_RESOLUTION
        )
        self.assertEqual(before, tuple(path.read_bytes() for path in paths))


if __name__ == "__main__":
    unittest.main()
