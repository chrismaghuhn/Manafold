from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import unittest
from dataclasses import replace
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import cast

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "src"))

from authority_source_resolver import AuthoritySourceResolver
from build_m2_5_c_authority_review_worklist import load_review_inputs
from build_m2_5_c_scaled_authority_pilot_proposal import (
    PILOT_SPECS,
    PilotProposalError,
    _candidate4_bridge,
    build_proposal_document,
    build_scaled_authority_pilot_proposal,
)


class ScaledAuthorityPilotProposalTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not os.environ.get("MANAFOLD_SOURCE_ARCHIVE"):
            raise unittest.SkipTest("MANAFOLD_SOURCE_ARCHIVE is required for source-bound pilot")
        cls.inputs = load_review_inputs(ROOT)
        cls.resolver = AuthoritySourceResolver(ROOT)

    def test_exact_five_candidate_lock_and_source_binding(self) -> None:
        proposal = build_proposal_document(ROOT, inputs=self.inputs, resolver=self.resolver)
        candidates = cast(dict[str, object], proposal["pilot"])["candidates"]
        self.assertEqual(len(candidates), 5)
        self.assertEqual(
            [item["candidate_id"] for item in candidates],
            [spec.candidate_id for spec in PILOT_SPECS],
        )
        for item, spec in zip(candidates, PILOT_SPECS, strict=True):
            self.assertEqual(item["candidate_identity"], spec.candidate_identity)
            self.assertEqual(item["source_instance_id"], spec.source_instance_id)
            self.assertEqual(item["rev3_row_ordinal"], spec.rev3_row_ordinal)
            self.assertEqual(
                item["candidate_universe_binding"]["raw_sha256"],
                self.inputs.candidate_universe_binding["raw_sha256"],
            )

    def test_candidate_substitution_fails_closed(self) -> None:
        bad_specs = list(PILOT_SPECS)
        bad_specs[0] = replace(
            bad_specs[0],
            candidate_id="INTRA_DECK|Token Triumph|cap.draw|cap.token_creation|UNORDERED_BINARY",
        )
        with self.assertRaises(PilotProposalError) as caught:
            build_proposal_document(
                ROOT,
                inputs=self.inputs,
                resolver=self.resolver,
                pilot_specs=tuple(bad_specs),
            )
        self.assertEqual(caught.exception.code, "PILOT_SELECTION_MISMATCH")

    def test_candidate4_source_instance_mutation_fails_closed(self) -> None:
        bad_specs = list(PILOT_SPECS)
        bad_specs[3] = replace(bad_specs[3], source_instance_id="si.v1/forged/0")
        with self.assertRaises(PilotProposalError) as caught:
            build_proposal_document(
                ROOT,
                inputs=self.inputs,
                resolver=self.resolver,
                pilot_specs=tuple(bad_specs),
            )
        self.assertEqual(caught.exception.code, "SOURCE_INSTANCE_BINDING_MISMATCH")

    def test_candidate4_identity_mutation_fails_closed(self) -> None:
        bad_specs = list(PILOT_SPECS)
        bad_specs[3] = replace(bad_specs[3], candidate_identity="0" * 64)
        with self.assertRaises(PilotProposalError) as caught:
            build_proposal_document(
                ROOT,
                inputs=self.inputs,
                resolver=self.resolver,
                pilot_specs=tuple(bad_specs),
            )
        self.assertEqual(caught.exception.code, "CANDIDATE_IDENTITY_MISMATCH")

    def test_candidate4_historical_role_mutation_fails_closed(self) -> None:
        source = next(
            item
            for item in self.inputs.source_instance_records
            if item["candidate_id"] == PILOT_SPECS[3].candidate_id
        )
        mutated = json.loads(json.dumps(source))
        cast(list[dict[str, object]], mutated["participant_bindings"])[0]["role"] = "source"
        with self.assertRaises(PilotProposalError) as caught:
            _candidate4_bridge(mutated, ("source", "affected"))
        self.assertEqual(caught.exception.code, "CANDIDATE4_HISTORICAL_ROLE_MISMATCH")

    def test_v1_v2_eligibility_is_not_inferred_for_unreviewed_candidates(self) -> None:
        proposal = build_proposal_document(ROOT, inputs=self.inputs, resolver=self.resolver)
        candidates = cast(dict[str, object], proposal["pilot"])["candidates"]
        for index in (0, 1, 2, 4):
            path = cast(dict[str, object], candidates[index]["proposed_authority_path"])
            self.assertEqual(path["status"], "blocked")
            self.assertEqual(path["eligibility"], "defer_until_exact_relation_proof_v1")
        candidate4_path = cast(dict[str, object], candidates[3]["proposed_authority_path"])
        self.assertEqual(candidate4_path["eligibility"], "role_divergent_requires_v2_v3")

    def test_wrong_candidate_universe_digest_fails_closed(self) -> None:
        bad_binding = dict(self.inputs.candidate_universe_binding)
        bad_binding["raw_sha256"] = "0" * 64
        bad_inputs = replace(self.inputs, candidate_universe_binding=bad_binding)
        with self.assertRaises(PilotProposalError):
            build_proposal_document(
                ROOT,
                inputs=bad_inputs,
                resolver=AuthoritySourceResolver(ROOT),
            )

    def test_missing_private_source_archive_is_blocked(self) -> None:
        previous = os.environ.pop("MANAFOLD_SOURCE_ARCHIVE", None)
        try:
            with self.assertRaises(PilotProposalError) as caught:
                build_proposal_document(
                    ROOT,
                    inputs=self.inputs,
                    resolver=AuthoritySourceResolver(ROOT),
                )
            self.assertEqual(caught.exception.status, "BLOCKED")
            self.assertEqual(caught.exception.code, "REV3_ARCHIVE_SOURCE_UNAVAILABLE")
        finally:
            if previous is not None:
                os.environ["MANAFOLD_SOURCE_ARCHIVE"] = previous

    def test_proposal_construction_does_not_mutate_protected_sources(self) -> None:
        def protected_digest() -> str:
            paths = subprocess.run(
                ["git", "ls-files", "sources/m2_5/authorities", "sources/m2_5/closures/C"],
                cwd=ROOT,
                check=True,
                capture_output=True,
                text=True,
            ).stdout.splitlines()
            digest = hashlib.sha256()
            for relative in paths:
                file = ROOT / relative
                digest.update(relative.encode("utf-8"))
                digest.update(file.read_bytes())
            return digest.hexdigest()

        before_status = subprocess.run(
            ["git", "status", "--porcelain=v1"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
        before_digest = protected_digest()
        with TemporaryDirectory() as temporary:
            build_scaled_authority_pilot_proposal(
                ROOT, Path(temporary), inputs=self.inputs, resolver=self.resolver
            )
        after_status = subprocess.run(
            ["git", "status", "--porcelain=v1"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
        self.assertEqual(after_status, before_status)
        self.assertEqual(protected_digest(), before_digest)

    def test_candidate4_uses_role_divergent_v2_v3_path_without_acceptance(self) -> None:
        proposal = build_proposal_document(ROOT, inputs=self.inputs, resolver=self.resolver)
        candidate4 = cast(dict[str, object], proposal["pilot"])["candidates"][3]
        path = cast(dict[str, object], candidate4["proposed_authority_path"])
        self.assertEqual(proposal["proposal_state"], "blocked")
        self.assertEqual(candidate4["semantic_status"], "blocked")
        self.assertEqual(path["status"], "blocked")
        self.assertEqual(path["relation_application_family"], "rpa.v2")
        self.assertEqual(path["context_application_family"], "cpa.v3")
        bridge = cast(list[dict[str, object]], path["participant_role_bridge"])
        self.assertEqual([entry["reviewed_role"] for entry in bridge], ["source", "affected"])
        self.assertEqual(proposal["authority_status"], "non_authoritative")
        self.assertEqual(proposal["acceptance_status"], "not_authorized")
        encoded = json.dumps(proposal, sort_keys=True)
        self.assertNotIn("human_accepted", encoded)
        self.assertNotIn("review_event_ref", encoded)

    def test_proposal_bytes_are_deterministic_and_quarantined(self) -> None:
        with TemporaryDirectory() as temporary:
            first = build_scaled_authority_pilot_proposal(
                ROOT, Path(temporary), inputs=self.inputs, resolver=self.resolver
            )
            first_bytes = first.read_bytes()
            second = build_scaled_authority_pilot_proposal(
                ROOT, Path(temporary), inputs=self.inputs, resolver=self.resolver
            )
            self.assertEqual(first, second)
            self.assertEqual(first_bytes, second.read_bytes())
            self.assertEqual(hashlib.sha256(first_bytes).hexdigest(), proposal_digest(first_bytes))
            self.assertNotIn("sources/m2_5/authorities/", first.as_posix())


def proposal_digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


if __name__ == "__main__":
    unittest.main()
