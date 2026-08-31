from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from copy import deepcopy
from dataclasses import replace
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from authority_source_resolver import (
    AuthoritySourceResolver,
    ResolutionError,
    ResolutionStatus,
)
from authority_validator import AuthorityValidator
from build_m2_5_c_authority_review_worklist import (
    ReviewWorklistError,
    build_worklist,
    load_review_inputs,
    validate_review_inputs,
)
from mtgml.authority import REVIEWER_ROSTER_SCHEMA_V1, ReviewerRosterRefV1
from scaffold_m2_5_c_authority_review import (
    scaffold_review_proposal,
)

WORKLIST_NAME = "review_worklist.v1.jsonl"
SUMMARY_NAME = "REVIEW_WORKLIST_SUMMARY.md"


class AuthorityReviewWorklistTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.inputs = load_review_inputs(ROOT)

    def test_baseline_inputs_are_complete_and_unresolved(self) -> None:
        self.assertEqual(len(self.inputs.candidate_records), 15679)
        self.assertEqual(len(self.inputs.source_instance_records), 15679)
        self.assertEqual(len(self.inputs.classification_records), 15679)
        self.assertEqual(len(self.inputs.review_domains), 11)
        self.assertEqual(
            sum(
                record["review_state"] == "unresolved"
                for record in self.inputs.classification_records
            ),
            15679,
        )

    def test_worklist_generation_is_byte_identical_on_repeat(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            first = root / "first"
            second = root / "second"
            build_worklist(ROOT, first, inputs=self.inputs)
            build_worklist(ROOT, second, inputs=self.inputs)
            first_worklist = (first / WORKLIST_NAME).read_bytes()
            second_worklist = (second / WORKLIST_NAME).read_bytes()
            self.assertEqual(first_worklist, second_worklist)
            self.assertEqual(
                hashlib.sha256(first_worklist).hexdigest(),
                hashlib.sha256(second_worklist).hexdigest(),
            )
            self.assertEqual(
                (first / SUMMARY_NAME).read_bytes(),
                (second / SUMMARY_NAME).read_bytes(),
            )

    def test_worklist_covers_candidates_in_existing_source_order(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            build_worklist(ROOT, output, inputs=self.inputs)
            lines = (output / WORKLIST_NAME).read_text(encoding="utf-8").splitlines()
        manifest = cast(dict[str, object], json.loads(lines[0]))
        items = [cast(dict[str, object], json.loads(line)) for line in lines[1:]]
        self.assertEqual(manifest["record_type"], "review_worklist_manifest")
        self.assertEqual(manifest["candidate_count"], 15679)
        self.assertEqual(manifest["work_item_count"], 15679)
        self.assertEqual([item["ordinal"] for item in items], list(range(15679)))
        self.assertEqual(
            [item["candidate_id"] for item in items],
            [record["candidate_id"] for record in self.inputs.classification_records],
        )
        self.assertTrue(all(item["current_review_state"] == "unresolved" for item in items))
        self.assertTrue(all("terminal_disposition" not in item for item in items))
        self.assertTrue(all("interaction_class_id" not in item for item in items))

    def test_worklist_manifest_binds_exact_sources(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            build_worklist(ROOT, output, inputs=self.inputs)
            manifest = cast(
                dict[str, object],
                json.loads((output / WORKLIST_NAME).read_text(encoding="utf-8").splitlines()[0]),
            )
        self.assertEqual(manifest["source_commit"], self.inputs.source_commit)
        self.assertEqual(manifest["declared_model"], self.inputs.model_binding)
        self.assertEqual(manifest["candidate_universe"], self.inputs.candidate_universe_binding)
        self.assertEqual(manifest["classification_root"], self.inputs.classification_root_binding)
        self.assertEqual(manifest["semantic_classes"], self.inputs.semantic_classes_binding)
        self.assertEqual(
            manifest["classification_shards"], list(self.inputs.classification_shard_bindings)
        )
        self.assertEqual(manifest["reviewer_roster"], self.inputs.reviewer_roster_ref)
        self.assertEqual(manifest["format"], "manafold.m2.5.c.authority-review-worklist.v1")

    def test_production_roster_is_the_verified_content_addressed_leaf(self) -> None:
        roster = self.inputs.reviewer_roster_ref
        roster_path = ROOT / Path(*cast(str, roster["path"]).split("/"))
        raw = roster_path.read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), roster["raw_sha256"])
        reference = ReviewerRosterRefV1(
            path=cast(str, roster["path"]),
            schema=REVIEWER_ROSTER_SCHEMA_V1,
            raw_sha256=bytes.fromhex(cast(str, roster["raw_sha256"])),
        )
        resolved = AuthoritySourceResolver(ROOT).resolve_reviewer_roster_leaf(reference)
        self.assertEqual(resolved.raw_bytes, raw)

    def test_changed_declared_model_source_fails_closed(self) -> None:
        binding = deepcopy(self.inputs.model_binding)
        binding["raw_sha256"] = "00" * 32
        mutated = replace(self.inputs, model_binding=binding)
        with self.assertRaises(ReviewWorklistError) as context:
            validate_review_inputs(mutated)
        self.assertEqual(context.exception.code, "SOURCE_BINDING_MISMATCH")

    def test_worklist_domain_obligations_use_model_order_without_prediction(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            build_worklist(ROOT, output, inputs=self.inputs)
            lines = (output / WORKLIST_NAME).read_text(encoding="utf-8").splitlines()
        item = cast(dict[str, object], json.loads(lines[1]))
        obligations = cast(dict[str, object], item["review_obligations"])
        domain_reviews = cast(list[dict[str, object]], obligations["domain_reviews"])
        self.assertEqual(
            [review["review_domain"] for review in domain_reviews],
            list(self.inputs.review_domains),
        )
        self.assertTrue(
            all(review["status"] == "awaiting_human_semantic_review" for review in domain_reviews)
        )
        self.assertEqual(
            obligations["conditional_context_review"], "defer_until_accepted_semantic_result"
        )
        self.assertEqual(
            obligations["conditional_scope_review"], "defer_until_accepted_semantic_result"
        )

    def test_duplicate_classification_candidate_fails_closed(self) -> None:
        records = list(self.inputs.classification_records)
        duplicate = deepcopy(records[0])
        duplicate["candidate_id"] = records[1]["candidate_id"]
        records[0] = duplicate
        mutated = replace(self.inputs, classification_records=tuple(records))
        with self.assertRaises(ReviewWorklistError) as context:
            validate_review_inputs(mutated)
        self.assertEqual(context.exception.code, "DUPLICATE_CLASSIFICATION_CANDIDATE")

    def test_duplicate_candidate_id_fails_closed(self) -> None:
        candidates = list(self.inputs.candidate_records)
        duplicate = deepcopy(candidates[1])
        duplicate["candidate_id"] = candidates[0]["candidate_id"]
        candidates[1] = duplicate
        mutated = replace(self.inputs, candidate_records=tuple(candidates))
        with self.assertRaises(ReviewWorklistError) as context:
            validate_review_inputs(mutated)
        self.assertEqual(context.exception.code, "DUPLICATE_CANDIDATE_ID")

    def test_candidate_classification_cardinality_fails_closed(self) -> None:
        root = deepcopy(self.inputs.classification_root)
        root["classification_count"] = 15678
        mutated = replace(self.inputs, classification_root=root)
        with self.assertRaises(ReviewWorklistError) as context:
            validate_review_inputs(mutated)
        self.assertEqual(context.exception.code, "CLASSIFICATION_CARDINALITY_MISMATCH")

    def test_source_digest_drift_fails_closed(self) -> None:
        root = deepcopy(self.inputs.classification_root)
        root["candidate_universe_raw_sha256"] = "00" * 32
        mutated = replace(self.inputs, classification_root=root)
        with self.assertRaises(ReviewWorklistError) as context:
            validate_review_inputs(mutated)
        self.assertEqual(context.exception.code, "SOURCE_BINDING_MISMATCH")

    def test_reviewer_roster_reference_drift_fails_closed(self) -> None:
        roster = deepcopy(self.inputs.reviewer_roster_ref)
        roster["raw_sha256"] = "00" * 32
        mutated = replace(self.inputs, reviewer_roster_ref=roster)
        with self.assertRaises(ReviewWorklistError) as context:
            validate_review_inputs(mutated)
        self.assertEqual(context.exception.code, "SOURCE_BINDING_MISMATCH")

    def test_candidate_classification_mismatch_fails_closed(self) -> None:
        records = list(self.inputs.classification_records)
        changed = deepcopy(records[0])
        changed["candidate_id"] = "missing-candidate"
        records[0] = changed
        mutated = replace(self.inputs, classification_records=tuple(records))
        with self.assertRaises(ReviewWorklistError) as context:
            validate_review_inputs(mutated)
        self.assertEqual(context.exception.code, "CLASSIFICATION_SHARD_SOURCE_MISMATCH")

    def test_resolved_classification_fails_closed(self) -> None:
        records = list(self.inputs.classification_records)
        changed = deepcopy(records[0])
        changed["review_state"] = "resolved"
        records[0] = changed
        mutated = replace(self.inputs, classification_records=tuple(records))
        with self.assertRaises(ReviewWorklistError) as context:
            validate_review_inputs(mutated)
        self.assertEqual(context.exception.code, "UNEXPECTED_REVIEW_STATE")

    def test_unknown_review_domain_fails_closed(self) -> None:
        records = list(self.inputs.classification_records)
        changed = deepcopy(records[0])
        assessments = deepcopy(cast(list[object], changed["review_domain_assessments"]))
        cast(dict[str, object], assessments[0])["review_domain"] = "unknown-domain"
        changed["review_domain_assessments"] = assessments
        records[0] = changed
        mutated = replace(self.inputs, classification_records=tuple(records))
        with self.assertRaises(ReviewWorklistError) as context:
            validate_review_inputs(mutated)
        self.assertEqual(context.exception.code, "UNKNOWN_REVIEW_DOMAIN")

    def test_incomplete_review_domain_coverage_fails_closed(self) -> None:
        records = list(self.inputs.classification_records)
        changed = deepcopy(records[0])
        changed["review_domain_assessments"] = cast(
            list[object], changed["review_domain_assessments"]
        )[:-1]
        records[0] = changed
        mutated = replace(self.inputs, classification_records=tuple(records))
        with self.assertRaises(ReviewWorklistError) as context:
            validate_review_inputs(mutated)
        self.assertEqual(context.exception.code, "REVIEW_DOMAIN_COVERAGE_MISMATCH")

    def test_missing_source_instance_fails_closed(self) -> None:
        mutated = replace(
            self.inputs, source_instance_records=self.inputs.source_instance_records[:-1]
        )
        with self.assertRaises(ReviewWorklistError) as context:
            validate_review_inputs(mutated)
        self.assertEqual(context.exception.code, "SOURCE_INSTANCE_CARDINALITY_MISMATCH")

    def test_proposal_scaffold_is_quarantined_from_authority(self) -> None:
        candidate_id = cast(str, self.inputs.candidate_records[0]["candidate_id"])
        with tempfile.TemporaryDirectory() as temporary:
            proposal_path = scaffold_review_proposal(
                ROOT, candidate_id, Path(temporary), inputs=self.inputs
            )
            proposal = cast(
                dict[str, object], json.loads(proposal_path.read_text(encoding="utf-8"))
            )
        self.assertEqual(proposal["format"], "manafold.m2.5.c.authority-review-proposal.v1")
        self.assertEqual(proposal["proposal_state"], "awaiting_human_semantic_review")
        self.assertEqual(proposal["authority_status"], "non_authoritative")
        encoded = json.dumps(proposal, sort_keys=True)
        for forbidden in (
            "human_accepted",
            "review_event_ref",
            "required_interaction",
            "not_an_interaction_with_proof",
            "out_of_declared_scope_with_reason",
            "positive_interaction",
            "positive_separation",
            "model_bound_scope",
        ):
            with self.subTest(forbidden=forbidden):
                self.assertNotIn(forbidden, encoded)
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(AuthoritySourceResolver(ROOT)).validate(proposal)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)

    def test_unknown_candidate_proposal_fails_closed(self) -> None:
        with (
            tempfile.TemporaryDirectory() as temporary,
            self.assertRaises(ReviewWorklistError) as context,
        ):
            scaffold_review_proposal(ROOT, "unknown-candidate", Path(temporary), inputs=self.inputs)
        self.assertEqual(context.exception.code, "CANDIDATE_NOT_FOUND")

    def test_worklist_tools_have_no_network_or_local_user_authority(self) -> None:
        for filename in (
            "scripts/build_m2_5_c_authority_review_worklist.py",
            "scripts/scaffold_m2_5_c_authority_review.py",
        ):
            source = (ROOT / filename).read_text(encoding="utf-8")
            for forbidden in ("github.com", "requests", "urllib", "getpass.getuser", "os.getlogin"):
                with self.subTest(filename=filename, forbidden=forbidden):
                    self.assertNotIn(forbidden, source)


if __name__ == "__main__":
    unittest.main()
