"""Shared typed reviewer-roster materialization and exact-role checks.

AuthoritySourceResolver owns raw roster bytes, digest/schema/path checks, and
closed JSON-shape validation. This module consumes only its verified JSON
projection and owns no reviewer ordering or acceptance policy.
"""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
from typing import Literal, cast

from authority_source_resolver import AuthoritySourceResolver
from mtgml.authority import (
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewerRosterV1,
    ReviewerV1,
)


@dataclass(frozen=True)
class ReviewerRoleBindingValidationError(ValueError):
    reason: Literal["not_in_roster", "role_mismatch"]
    reviewer_id: str | None = None


def resolve_reviewer_roster(
    source_resolver: AuthoritySourceResolver,
    reference: ReviewerRosterRefV1,
) -> ReviewerRosterV1:
    """Resolve one verified roster artifact and materialize its typed value."""

    artifact = source_resolver.resolve_reviewer_roster_leaf(reference)
    value = cast(Mapping[str, object], artifact.json_value)
    raw_reviewers = cast(list[object], value["reviewers"])
    reviewers = tuple(
        ReviewerV1(
            cast(str, cast(Mapping[str, object], raw_reviewer)["reviewer_id"]),
            tuple(cast(list[str], cast(Mapping[str, object], raw_reviewer)["roles"])),
        )
        for raw_reviewer in raw_reviewers
    )
    return ReviewerRosterV1(reviewers)


def validate_reviewer_binding_against_roster(
    binding: ReviewerRoleBindingV1,
    roster: ReviewerRosterV1,
) -> None:
    """Require reviewer existence and complete roster-role equality."""

    roster_by_id = {reviewer.reviewer_id: reviewer for reviewer in roster.reviewers}
    reviewer = roster_by_id.get(binding.reviewer_id)
    if reviewer is None:
        raise ReviewerRoleBindingValidationError("not_in_roster", binding.reviewer_id)
    if tuple(reviewer.roles) != binding.roles:
        raise ReviewerRoleBindingValidationError("role_mismatch", binding.reviewer_id)


__all__ = [
    "ReviewerRoleBindingValidationError",
    "resolve_reviewer_roster",
    "validate_reviewer_binding_against_roster",
]
