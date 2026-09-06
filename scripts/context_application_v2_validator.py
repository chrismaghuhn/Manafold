"""Semantic validation for ContextApplicationV2 Slice 3.

The pure portion of this module compares already typed values only. The
filesystem-aware validator is added below that core and composes the existing
source resolvers without becoming a rules engine.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Final

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from mtgml.authority import ContextBridgeRelationV2
from mtgml.persistence import PersistenceValue

CONTEXT_SLOT_COUNT: Final = 10
TEMPORAL_SLOT_COUNT: Final = 4
CONTEXT_DIMENSIONS: Final = (
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
TEMPORAL_SEMANTICS: Final = (
    "trigger_order",
    "dependency_order",
    "duration",
    "replacement_order",
)


@dataclass(frozen=True)
class ContextPreconditionValueV1:
    precondition_id: str
    value: PersistenceValue


class ContextApplicationV2SemanticValidationError(ValueError):
    """A fail-closed semantic mismatch with a stable category and location."""

    def __init__(self, code: str, location: str) -> None:
        self.code = code
        self.location = location
        super().__init__(f"{code} at {location}")


@dataclass(frozen=True)
class ContextApplicationV2SemanticValidationResult:
    valid: bool
    error_code: str | None = None


@dataclass(frozen=True)
class ContextApplicationV2SemanticInput:
    theorem_subject_shape: PersistenceValue
    member_context_binding: PersistenceValue
    historical_source_values: tuple[str, ...]
    bridge_source_values: tuple[str, ...]
    theorem_context_values: tuple[str, ...]
    bridge_reviewed_values: tuple[str, ...]
    bridge_relations: tuple[ContextBridgeRelationV2, ...]
    theorem_temporal_values: tuple[str, ...]
    bridge_temporal_values: tuple[str, ...]
    theorem_preconditions: tuple[ContextPreconditionValueV1, ...]
    member_preconditions: tuple[ContextPreconditionValueV1, ...]


def validate_context_application_v2_semantics(
    value: ContextApplicationV2SemanticInput,
) -> ContextApplicationV2SemanticValidationResult:
    """Validate exact V2 bridge equations without accessing any source."""

    if value.member_context_binding != value.theorem_subject_shape:
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_SUBJECT_MISMATCH", "member_context_binding"
        )
    if len(value.historical_source_values) != CONTEXT_SLOT_COUNT:
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_SOURCE_CONTEXT_MISMATCH", "historical_source_values"
        )
    if len(value.bridge_source_values) != CONTEXT_SLOT_COUNT:
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_SOURCE_CONTEXT_MISMATCH", "bridge_source_values"
        )
    if len(value.theorem_context_values) != CONTEXT_SLOT_COUNT:
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_REVIEWED_CONTEXT_MISMATCH", "theorem_context_values"
        )
    if len(value.bridge_reviewed_values) != CONTEXT_SLOT_COUNT:
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_REVIEWED_CONTEXT_MISMATCH", "bridge_reviewed_values"
        )
    if len(value.bridge_relations) != CONTEXT_SLOT_COUNT:
        raise ContextApplicationV2SemanticValidationError(
            "BRIDGE_RELATION_MISMATCH", "bridge_relations"
        )

    for index, (historical, source) in enumerate(
        zip(value.historical_source_values, value.bridge_source_values, strict=True)
    ):
        if historical != source:
            raise ContextApplicationV2SemanticValidationError(
                "MEMBER_SOURCE_CONTEXT_MISMATCH", f"context[{index}]"
            )

    for index, (theorem, reviewed) in enumerate(
        zip(value.theorem_context_values, value.bridge_reviewed_values, strict=True)
    ):
        if theorem != reviewed:
            raise ContextApplicationV2SemanticValidationError(
                "MEMBER_REVIEWED_CONTEXT_MISMATCH", f"context[{index}]"
            )

    for index, (source, reviewed, relation) in enumerate(
        zip(
            value.bridge_source_values,
            value.bridge_reviewed_values,
            value.bridge_relations,
            strict=True,
        )
    ):
        expected = (
            ContextBridgeRelationV2.EXACT_MATCH
            if source == reviewed
            else ContextBridgeRelationV2.REVIEWED_DIVERGENCE
        )
        if relation is not expected:
            raise ContextApplicationV2SemanticValidationError(
                "BRIDGE_RELATION_MISMATCH", f"context[{index}]"
            )

    if len(value.theorem_temporal_values) != TEMPORAL_SLOT_COUNT:
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_TEMPORAL_MISMATCH", "theorem_temporal_values"
        )
    if len(value.bridge_temporal_values) != TEMPORAL_SLOT_COUNT:
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_TEMPORAL_MISMATCH", "bridge_temporal_values"
        )
    for index, (theorem, reviewed) in enumerate(
        zip(value.theorem_temporal_values, value.bridge_temporal_values, strict=True)
    ):
        if theorem != reviewed:
            raise ContextApplicationV2SemanticValidationError(
                "MEMBER_TEMPORAL_MISMATCH", f"temporal[{index}]"
            )

    if len(value.theorem_preconditions) != len(value.member_preconditions):
        raise ContextApplicationV2SemanticValidationError(
            "PRECONDITION_COVERAGE", "preconditions"
        )
    for index, (theorem, member) in enumerate(
        zip(value.theorem_preconditions, value.member_preconditions, strict=True)
    ):
        if (
            theorem.precondition_id != member.precondition_id
            or theorem.value != member.value
        ):
            raise ContextApplicationV2SemanticValidationError(
                "PRECONDITION_MISMATCH", f"preconditions[{index}]"
            )

    return ContextApplicationV2SemanticValidationResult(valid=True)


__all__ = [
    "CONTEXT_DIMENSIONS",
    "CONTEXT_SLOT_COUNT",
    "ContextApplicationV2SemanticInput",
    "ContextApplicationV2SemanticValidationError",
    "ContextApplicationV2SemanticValidationResult",
    "ContextPreconditionValueV1",
    "TEMPORAL_SEMANTICS",
    "TEMPORAL_SLOT_COUNT",
    "validate_context_application_v2_semantics",
]
