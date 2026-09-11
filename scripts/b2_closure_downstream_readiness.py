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
from types import MappingProxyType
from typing import cast

from b2_closure_source_resolver import (
    B2ClosureResolutionMode,
    B2ClosureSourceBinding,
    B2ClosureSourceResolution,
    binding_for_mode,
    resolve_b2_closure_binding,
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

    AUTHORITY_SOURCE_RESOLUTION = "authority_source_resolution"
    AUTHORITY_VALIDATOR = "authority_validator"
    B1_CURRENT_EVIDENCE_ROOT = "b1_current_evidence_root"
    C_CURRENT_SOURCE_ROOT = "c_current_source_root"
    RELATION_APPLICATION = "relation_application"
    CONTEXT_APPLICATION = "context_application"
    HOST_BINDING = "host_binding"
    REVIEW_ACCEPTANCE = "review_acceptance"


class B2DownstreamReadinessStatus(StrEnum):
    ALREADY_READY = "already_ready"
    NOT_APPLICABLE = "not_applicable"


@dataclass(frozen=True)
class B2DownstreamOwnership:
    status: B2DownstreamReadinessStatus
    rationale: str


B2_DOWNSTREAM_OWNERSHIP = MappingProxyType(
    {
        B2DownstreamReadinessConsumer.AUTHORITY_SOURCE_RESOLUTION: B2DownstreamOwnership(
            B2DownstreamReadinessStatus.ALREADY_READY,
            "Slice 3 owns explicit verified historical-v1/candidate-v2 source resolution.",
        ),
        B2DownstreamReadinessConsumer.AUTHORITY_VALIDATOR: B2DownstreamOwnership(
            B2DownstreamReadinessStatus.ALREADY_READY,
            "AuthorityValidator owns the non-current successor readiness entrypoint.",
        ),
        B2DownstreamReadinessConsumer.B1_CURRENT_EVIDENCE_ROOT: B2DownstreamOwnership(
            B2DownstreamReadinessStatus.ALREADY_READY,
            "The B1 citation checker owns the non-current evidence-root readiness entrypoint.",
        ),
        B2DownstreamReadinessConsumer.C_CURRENT_SOURCE_ROOT: B2DownstreamOwnership(
            B2DownstreamReadinessStatus.ALREADY_READY,
            "The RelationApplication resolver owns the non-current C source-root "
            "readiness entrypoint.",
        ),
        B2DownstreamReadinessConsumer.RELATION_APPLICATION: B2DownstreamOwnership(
            B2DownstreamReadinessStatus.ALREADY_READY,
            "The RelationApplication resolver owns the non-current successor readiness entrypoint.",
        ),
        B2DownstreamReadinessConsumer.CONTEXT_APPLICATION: B2DownstreamOwnership(
            B2DownstreamReadinessStatus.NOT_APPLICABLE,
            "Historical ContextApplication contracts remain frozen; successor admission "
            "is not yet owned.",
        ),
        B2DownstreamReadinessConsumer.HOST_BINDING: B2DownstreamOwnership(
            B2DownstreamReadinessStatus.NOT_APPLICABLE,
            "Historical HostBinding contracts remain frozen; successor admission is not yet owned.",
        ),
        B2DownstreamReadinessConsumer.REVIEW_ACCEPTANCE: B2DownstreamOwnership(
            B2DownstreamReadinessStatus.NOT_APPLICABLE,
            "No production acceptance-event successor is authorized in Slice 4.",
        ),
    }
)


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


def require_verified_b2_v2(
    repo_root: Path, binding: B2ClosureSourceBinding
) -> B2ClosureSourceResolution:
    """Shared owner primitive for an exact, explicitly supplied v2 witness."""

    resolution = resolve_b2_closure_binding(repo_root, binding)
    if resolution.mode is not B2ClosureResolutionMode.CANDIDATE_V2:
        raise B2DownstreamReadinessError(
            "CURRENT_CONSUMER_VERSION_UNSUPPORTED",
            "successor readiness requires candidate-v2 closure resolution",
        )
    if not resolution.v2_verified or resolution.v2_current:
        raise B2DownstreamReadinessError(
            "B2_DOWNSTREAM_READINESS_INVALID",
            "successor readiness requires verified non-current closure v2",
        )
    return resolution


def _owner_resolution(
    repo_root: Path,
    consumer: B2DownstreamReadinessConsumer,
    binding: B2ClosureSourceBinding,
) -> B2ClosureSourceResolution:
    if consumer is B2DownstreamReadinessConsumer.AUTHORITY_SOURCE_RESOLUTION:
        return require_verified_b2_v2(repo_root, binding)
    if consumer is B2DownstreamReadinessConsumer.AUTHORITY_VALIDATOR:
        from authority_validator import AuthorityValidator

        return AuthorityValidator.validate_b2_closure_v2_readiness(repo_root, binding)
    if consumer is B2DownstreamReadinessConsumer.B1_CURRENT_EVIDENCE_ROOT:
        from check_m2_5_b1_authority_citations import (
            validate_b2_closure_v2_readiness,
        )

        return validate_b2_closure_v2_readiness(repo_root, binding)
    if consumer in {
        B2DownstreamReadinessConsumer.C_CURRENT_SOURCE_ROOT,
        B2DownstreamReadinessConsumer.RELATION_APPLICATION,
    }:
        from relation_application_v2_resolver import RelationApplicationV2Resolver

        return RelationApplicationV2Resolver.validate_b2_closure_v2_readiness(repo_root, binding)
    raise B2DownstreamReadinessError(
        "DOWNSTREAM_CONSUMER_NOT_APPLICABLE",
        B2_DOWNSTREAM_OWNERSHIP[consumer].rationale,
    )


def validate_b2_downstream_readiness(
    repo_root: Path,
    readiness: B2DownstreamReadiness,
) -> B2DownstreamReadiness:
    """Validate a readiness input without selecting or creating current state."""

    if not isinstance(readiness, B2DownstreamReadiness):
        raise B2DownstreamReadinessError(
            "B2_DOWNSTREAM_READINESS_INVALID", "readiness has the wrong type"
        )
    consumer = _coerce_consumer(readiness.consumer)
    ownership = B2_DOWNSTREAM_OWNERSHIP[consumer]
    if ownership.status is not B2DownstreamReadinessStatus.ALREADY_READY:
        raise B2DownstreamReadinessError("DOWNSTREAM_CONSUMER_NOT_APPLICABLE", ownership.rationale)
    resolution = readiness.resolution
    if resolution.v2_current:
        raise B2DownstreamReadinessError(
            "CURRENT_ROOT_ADOPTION_FORBIDDEN",
            "Slice 4 readiness cannot claim that closure v2 is current",
        )
    fresh_resolution = _owner_resolution(repo_root, consumer, resolution.binding)
    if fresh_resolution != resolution:
        raise B2DownstreamReadinessError(
            "B2_DOWNSTREAM_WITNESS_MISMATCH",
            "readiness witness does not equal a fresh repository-verified resolution",
        )
    if resolution.mode is not B2ClosureResolutionMode.CANDIDATE_V2:
        raise B2DownstreamReadinessError(
            "CURRENT_CONSUMER_VERSION_UNSUPPORTED",
            "downstream successor readiness requires the explicit candidate-v2 resolution",
        )
    if not resolution.v2_verified:
        raise B2DownstreamReadinessError(
            "B2_CLOSURE_NOT_VERIFIED", "downstream readiness requires verified closure v2"
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
    if B2_DOWNSTREAM_OWNERSHIP[normalized].status is not B2DownstreamReadinessStatus.ALREADY_READY:
        raise B2DownstreamReadinessError(
            "DOWNSTREAM_CONSUMER_NOT_APPLICABLE",
            B2_DOWNSTREAM_OWNERSHIP[normalized].rationale,
        )
    binding = binding_for_mode(B2ClosureResolutionMode.CANDIDATE_V2)
    resolution = _owner_resolution(repo_root, normalized, binding)
    return validate_b2_downstream_readiness(
        repo_root, B2DownstreamReadiness(consumer=normalized, resolution=resolution)
    )


__all__ = [
    "B2_DOWNSTREAM_OWNERSHIP",
    "B2DownstreamOwnership",
    "B2DownstreamReadiness",
    "B2DownstreamReadinessConsumer",
    "B2DownstreamReadinessError",
    "B2DownstreamReadinessStatus",
    "prepare_b2_downstream_readiness",
    "require_verified_b2_v2",
    "validate_b2_downstream_readiness",
]
