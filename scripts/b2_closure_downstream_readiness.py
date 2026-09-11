"""Non-current downstream readiness for the ADR 0046 B2 v2 closure.

This is an internal readiness seam, not a persisted Authority contract and
not a current-root selector.  It accepts only the explicitly verified Slice-3
candidate-v2 resolution; historical consumers and their role registries remain
unchanged.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import cast

from b2_closure_source_resolver import (
    B2ClosureResolutionMode,
    B2ClosureSourceResolution,
    resolve_b2_closure,
)


class B2DownstreamReadinessError(Exception):
    """A fail-closed error at the non-current downstream readiness boundary."""

    def __init__(self, code: str, message: str) -> None:
        self.status = "FAIL"
        self.code = code
        self.message = message
        super().__init__(f"[FAIL:{code}] {message}")


class B2DownstreamReadinessConsumer(StrEnum):
    """Future current-construction seams covered by this readiness proof."""

    AUTHORITY_VALIDATOR = "authority_validator"
    B1_CURRENT_EVIDENCE_ROOT = "b1_current_evidence_root"
    C_CURRENT_SOURCE_ROOT = "c_current_source_root"
    RELATION_APPLICATION = "relation_application"
    CONTEXT_APPLICATION = "context_application"
    HOST_BINDING = "host_binding"
    REVIEW_ACCEPTANCE = "review_acceptance"


@dataclass(frozen=True)
class B2DownstreamReadiness:
    """A verified future-consumer input that is explicitly non-current."""

    consumer: B2DownstreamReadinessConsumer
    resolution: B2ClosureSourceResolution

    @property
    def v2_ready(self) -> bool:
        return (
            self.resolution.mode is B2ClosureResolutionMode.CANDIDATE_V2
            and self.resolution.v2_verified
        )

    @property
    def v2_current(self) -> bool:
        return cast(bool, self.resolution.v2_current)

    @property
    def production_record_created(self) -> bool:
        return False


def _coerce_consumer(
    consumer: B2DownstreamReadinessConsumer | str,
) -> B2DownstreamReadinessConsumer:
    if isinstance(consumer, B2DownstreamReadinessConsumer):
        return consumer
    try:
        return B2DownstreamReadinessConsumer(consumer)
    except (TypeError, ValueError) as exc:
        raise B2DownstreamReadinessError(
            "UNKNOWN_DOWNSTREAM_CONSUMER",
            f"unknown downstream readiness consumer: {consumer!r}",
        ) from exc


def validate_b2_downstream_readiness(
    readiness: B2DownstreamReadiness,
) -> B2DownstreamReadiness:
    """Validate a readiness input without selecting or creating current state."""

    if not isinstance(readiness, B2DownstreamReadiness):
        raise B2DownstreamReadinessError(
            "B2_DOWNSTREAM_READINESS_INVALID", "readiness has the wrong type"
        )
    _coerce_consumer(readiness.consumer)
    resolution = readiness.resolution
    if resolution.mode is not B2ClosureResolutionMode.CANDIDATE_V2:
        raise B2DownstreamReadinessError(
            "CURRENT_CONSUMER_VERSION_UNSUPPORTED",
            "downstream successor readiness requires the explicit candidate-v2 resolution",
        )
    if not resolution.v2_verified:
        raise B2DownstreamReadinessError(
            "B2_CLOSURE_NOT_VERIFIED", "downstream readiness requires verified closure v2"
        )
    if resolution.v2_current:
        raise B2DownstreamReadinessError(
            "CURRENT_ROOT_ADOPTION_FORBIDDEN",
            "Slice 4 readiness cannot claim that closure v2 is current",
        )
    if resolution.binding.artifact_role != "b2_closure_v2":
        raise B2DownstreamReadinessError(
            "CURRENT_CONSUMER_ROLE_MISMATCH",
            "successor readiness requires artifact role b2_closure_v2",
        )
    return readiness


def prepare_b2_downstream_readiness(
    repo_root: Path,
    consumer: B2DownstreamReadinessConsumer | str,
) -> B2DownstreamReadiness:
    """Prepare one explicit v2-ready, non-current downstream input.

    The function intentionally has no version argument and no fallback path.
    Later current construction must obtain currentness from the sole current
    root in Slice 6.
    """

    normalized = _coerce_consumer(consumer)
    resolution = resolve_b2_closure(repo_root, B2ClosureResolutionMode.CANDIDATE_V2)
    return validate_b2_downstream_readiness(
        B2DownstreamReadiness(consumer=normalized, resolution=resolution)
    )


__all__ = [
    "B2DownstreamReadiness",
    "B2DownstreamReadinessConsumer",
    "B2DownstreamReadinessError",
    "prepare_b2_downstream_readiness",
    "validate_b2_downstream_readiness",
]
