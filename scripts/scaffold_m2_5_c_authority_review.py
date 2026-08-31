"""Create one quarantined, non-authoritative M2.5.C review proposal."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Final, cast

from build_m2_5_c_authority_review_worklist import (
    ROOT,
    LoadedReviewInputs,
    ReviewWorklistError,
    _json_bytes,
    _plain,
    _review_obligations,
    _text,
    load_review_inputs,
    validate_review_inputs,
)

PROPOSAL_FORMAT: Final = "manafold.m2.5.c.authority-review-proposal.v1"


def scaffold_review_proposal(
    repo_root: Path,
    candidate_id: str,
    output_dir: Path | None = None,
    *,
    inputs: LoadedReviewInputs | None = None,
) -> Path:
    """Write one proposal skeleton without creating any semantic authority."""

    loaded = inputs or load_review_inputs(repo_root)
    validate_review_inputs(loaded)
    candidates = {
        _text(candidate.get("candidate_id"), "candidate ID"): candidate
        for candidate in loaded.candidate_records
    }
    candidate = candidates.get(candidate_id)
    if candidate is None:
        raise ReviewWorklistError(
            "CANDIDATE_NOT_FOUND", f"candidate {candidate_id!r} is not in the worklist"
        )
    instances = {
        _text(instance.get("candidate_id"), "source-instance candidate ID"): instance
        for instance in loaded.source_instance_records
    }
    classifications = {
        _text(record.get("candidate_id"), "classification candidate ID"): record
        for record in loaded.classification_records
    }
    instance = instances[candidate_id]
    classification = classifications[candidate_id]
    ordinal = next(
        index
        for index, item in enumerate(loaded.classification_records)
        if item.get("candidate_id") == candidate_id
    )
    proposal = {
        "record_type": "non_authoritative_review_proposal",
        "format": PROPOSAL_FORMAT,
        "proposal_state": "awaiting_human_semantic_review",
        "authority_status": "non_authoritative",
        "source_identity": {
            "source_commit": loaded.source_commit,
            "declared_model": _plain(loaded.model_binding),
            "candidate_universe": _plain(loaded.candidate_universe_binding),
            "current_c_closure": _plain(loaded.current_c_closure_binding),
            "classification_root": _plain(loaded.classification_root_binding),
            "reviewer_roster": _plain(loaded.reviewer_roster_ref),
        },
        "candidate": {
            "ordinal": ordinal,
            "candidate_id": candidate["candidate_id"],
            "candidate_identity": _plain(candidate["candidate_identity"]),
            "source_instance_id": instance["source_instance_id"],
            "scope": candidate["scope"],
            "relation": candidate["relation"],
            "participant_refs": _plain(candidate["participant_refs"]),
            "candidate_source_binding": _plain(candidate["source_binding"]),
            "source_instance_binding": _plain(instance["source_binding"]),
        },
        "current_review": {
            "review_state": classification["review_state"],
            "unresolved_reason": classification["unresolved_reason"],
        },
        "review_obligations": _review_obligations(loaded.review_domains),
        "open_human_semantic_fields": [
            "relation_proof",
            "domain_applicability",
            "context_values",
            "scope_result",
            "terminal_disposition",
            "semantic_class",
        ],
    }
    target_dir = output_dir or repo_root / "dist" / "m2-5-c-authority-review" / "proposals"
    target_dir.mkdir(parents=True, exist_ok=True)
    identity = _text(
        cast(dict[str, object], candidate["candidate_identity"]).get("digest_hex"),
        "candidate identity digest",
    )
    target = target_dir / f"proposal-{identity}.v1.json"
    target.write_bytes(_json_bytes(proposal))
    return target


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-id", required=True)
    parser.add_argument("--output-dir", type=Path, default=None)
    args = parser.parse_args()
    try:
        path = scaffold_review_proposal(ROOT, args.candidate_id, args.output_dir)
    except ReviewWorklistError as exc:
        print(f"{exc.status}: {exc.code}: {exc.message}", file=sys.stderr)
        return 2 if exc.status == "BLOCKED" else 1
    print(f"PASS: generated non-authoritative proposal {path.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
