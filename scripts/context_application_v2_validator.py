"""Semantic validation for ContextApplicationV2 Slice 3.

The pure portion of this module compares already typed values only. The
filesystem-aware validator is added below that core and composes the existing
source resolvers without becoming a rules engine.
"""

from __future__ import annotations

import re
import sys
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Final, cast

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


@dataclass(frozen=True)
class ContextApplicationV2InformationSensitivityInventory:
    fact_paths: tuple[str, ...]


def collect_information_sensitivity_facts(
    value: ContextApplicationV2SemanticInput,
) -> ContextApplicationV2InformationSensitivityInventory:
    """Collect typed visibility/information facts without validating semantics."""

    fact_paths: list[str] = []
    information_indexes = ((1, "visibility"), (8, "information_relation"))

    for index, name in information_indexes:
        if value.historical_source_values[index] != "not_applicable":
            fact_paths.append(f"historical_source.{name}")
        if value.bridge_source_values[index] != "not_applicable":
            fact_paths.append(f"bridge.source.{name}")
        if value.bridge_reviewed_values[index] != "not_applicable":
            fact_paths.append(f"bridge.reviewed.{name}")
        if value.theorem_context_values[index] != "not_applicable":
            fact_paths.append(f"theorem.context.{name}")

    for precondition in value.theorem_preconditions:
        payload = precondition.value
        if (
            isinstance(payload, list)
            and len(payload) == 2
            and payload[0] in {"visibility", "information_relation"}
            and payload[1] != "not_applicable"
        ):
            fact_paths.append(
                f"precondition[{precondition.precondition_id}].source_context.{payload[0]}"
            )
            continue
        if not isinstance(payload, list) or len(payload) != 9:
            continue
        context_values = payload[4]
        if not isinstance(context_values, list) or len(context_values) != len(CONTEXT_DIMENSIONS):
            continue
        for index, name in information_indexes:
            if context_values[index] != "not_applicable":
                fact_paths.append(
                    f"precondition[{precondition.precondition_id}].class_projection.{name}"
                )

    return ContextApplicationV2InformationSensitivityInventory(tuple(fact_paths))


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
        raise ContextApplicationV2SemanticValidationError("PRECONDITION_COVERAGE", "preconditions")
    for index, (theorem, member) in enumerate(
        zip(value.theorem_preconditions, value.member_preconditions, strict=True)
    ):
        if theorem.precondition_id != member.precondition_id or theorem.value != member.value:
            raise ContextApplicationV2SemanticValidationError(
                "PRECONDITION_MISMATCH", f"preconditions[{index}]"
            )

    return ContextApplicationV2SemanticValidationResult(valid=True)


def _required_array(value: object, label: str) -> list[object]:
    if not isinstance(value, list):
        raise ContextApplicationV2SemanticValidationError("PRECONDITION_MISMATCH", label)
    return value


def _required_text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise ContextApplicationV2SemanticValidationError("THEOREM_REFERENCE_INVALID", label)
    return value


def _context_binding_from_v1_wire(
    value: object,
    label: str,
) -> PersistenceValue:
    if not isinstance(value, Mapping) or set(value) != {
        "arity",
        "directionality",
        "participant_roles",
        "host_relationship",
    }:
        raise ContextApplicationV2SemanticValidationError("MEMBER_SUBJECT_MISMATCH", label)
    participant_values: list[PersistenceValue] = []
    for index, participant in enumerate(
        _required_array(value.get("participant_roles"), f"{label}.participant_roles")
    ):
        if not isinstance(participant, Mapping) or set(participant) != {
            "position",
            "role",
            "participant_kind",
            "semantic_ref",
        }:
            raise ContextApplicationV2SemanticValidationError(
                "MEMBER_SUBJECT_MISMATCH", f"{label}.participant_roles[{index}]"
            )
        participant_values.append(
            [
                cast(PersistenceValue, participant["position"]),
                cast(PersistenceValue, participant["role"]),
                cast(PersistenceValue, participant["participant_kind"]),
                cast(PersistenceValue, participant["semantic_ref"]),
            ]
        )
    return [
        cast(PersistenceValue, value["arity"]),
        cast(PersistenceValue, value["directionality"]),
        participant_values,
        cast(PersistenceValue, value["host_relationship"]),
    ]


def _context_binding_to_v1_wire(
    value: object,
    label: str,
) -> dict[str, object]:
    if not isinstance(value, list) or len(value) != 4:
        raise ContextApplicationV2SemanticValidationError("MEMBER_SUBJECT_MISMATCH", label)
    raw_participants = value[2]
    if not isinstance(raw_participants, list):
        raise ContextApplicationV2SemanticValidationError(
            "MEMBER_SUBJECT_MISMATCH", f"{label}.participant_roles"
        )
    participants: list[dict[str, object]] = []
    for index, raw_participant in enumerate(raw_participants):
        if not isinstance(raw_participant, list) or len(raw_participant) != 4:
            raise ContextApplicationV2SemanticValidationError(
                "MEMBER_SUBJECT_MISMATCH", f"{label}.participant_roles[{index}]"
            )
        participants.append(
            {
                "position": raw_participant[0],
                "role": raw_participant[1],
                "participant_kind": raw_participant[2],
                "semantic_ref": raw_participant[3],
            }
        )
    return {
        "arity": value[0],
        "directionality": value[1],
        "participant_roles": participants,
        "host_relationship": value[3],
    }


def _theorem_preconditions(
    theorem: Mapping[str, object], label: str
) -> tuple[ContextPreconditionValueV1, ...]:
    result: list[ContextPreconditionValueV1] = []
    for index, raw in enumerate(_required_array(theorem.get("preconditions"), label)):
        if not isinstance(raw, Mapping):
            raise ContextApplicationV2SemanticValidationError(
                "PRECONDITION_MISMATCH", f"{label}.preconditions[{index}]"
            )
        result.append(
            ContextPreconditionValueV1(
                precondition_id=_required_text(
                    raw.get("precondition_id"),
                    f"{label}.preconditions[{index}].precondition_id",
                ),
                value=cast(PersistenceValue, raw.get("payload")),
            )
        )
    return tuple(result)


def _member_preconditions(member: object, label: str) -> tuple[ContextPreconditionValueV1, ...]:
    raw_attestations = getattr(member, "precondition_attestations_v1", None)
    if not isinstance(raw_attestations, list):
        raise ContextApplicationV2SemanticValidationError(
            "PRECONDITION_MISMATCH", f"{label}.preconditions"
        )
    result: list[ContextPreconditionValueV1] = []
    for index, raw in enumerate(raw_attestations):
        if not isinstance(raw, list) or len(raw) != 4:
            raise ContextApplicationV2SemanticValidationError(
                "PRECONDITION_MISMATCH", f"{label}.preconditions[{index}]"
            )
        result.append(
            ContextPreconditionValueV1(
                precondition_id=_required_text(
                    raw[0], f"{label}.preconditions[{index}].precondition_id"
                ),
                value=cast(PersistenceValue, raw[1]),
            )
        )
    return tuple(result)


@dataclass(frozen=True)
class ContextApplicationV2ValidationResult:
    valid: bool
    member_count: int


_CANDIDATE_POINTER_ROOT = re.compile(r"^/candidates/([0-9]+)(?:/.*)?$")
_SOURCE_INSTANCE_POINTER_ROOT = re.compile(r"^/source_instances/([0-9]+)(?:/.*)?$")
_CANDIDATE_IDENTITY_FIELDS = frozenset(
    {
        "algorithm_id",
        "digest_hex",
        "envelope_id",
        "input_schema_id",
        "payload_codec_id",
        "semantic_domain",
    }
)
_CANDIDATE_DIGEST_HEX = re.compile(r"^[0-9a-f]{64}$")


class ContextApplicationV2SemanticValidator:
    """Compose exact source resolution with the Slice-3 semantic core."""

    def __init__(
        self,
        source_resolver: object,
        *,
        base_authority_binding: object,
    ) -> None:
        from authority_source_resolver import AuthoritySourceResolver
        from context_application_v2_resolver import ContextApplicationV2Resolver

        if not isinstance(source_resolver, AuthoritySourceResolver):
            raise ContextApplicationV2SemanticValidationError(
                "APPLICATION_INPUT_INVALID", "source_resolver"
            )
        self._source_resolver = source_resolver
        self._base_binding = base_authority_binding
        self._v2_resolver = ContextApplicationV2Resolver(
            source_resolver,
            base_authority_binding=base_authority_binding,
        )

    def validate(self, application: object) -> ContextApplicationV2ValidationResult:
        from authority_source_resolver import ResolutionError, ResolvedSourceInstance
        from authority_validator import AuthorityValidator
        from context_application_v2_resolver import (
            ContextApplicationV2ResolutionError,
        )
        from mtgml.authority import (
            AuthorityIdentityKind,
            AuthorityIdentityV1,
            ContextApplicationV2InputV1,
            ContextApplicationV2Record,
            ContextApplicationV2RecordInputV1,
            ContextAuthoritySourceBindingV2,
            RecordKind,
        )

        if not isinstance(application, ContextApplicationV2Record | ContextApplicationV2InputV1):
            raise ContextApplicationV2SemanticValidationError(
                "APPLICATION_INPUT_INVALID", "application"
            )
        if not isinstance(self._base_binding, ContextAuthoritySourceBindingV2):
            raise ContextApplicationV2SemanticValidationError(
                "APPLICATION_INPUT_INVALID", "base_authority_binding"
            )

        if isinstance(application, ContextApplicationV2Record):
            theorem_record_id = application.theorem_record_id
            members = application.members
            application_id = application.application_id
            record_id = application.record_id
        else:
            theorem_record_id = AuthorityIdentityV1(
                AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
                application.theorem_record_id_bytes,
            )
            members = application.members
            application_id = application.identity()
            record_id = None

        base_artifact = self._v2_resolver.resolve_source_binding(self._base_binding)
        if not isinstance(base_artifact.json_value, Mapping):
            raise ContextApplicationV2SemanticValidationError(
                "THEOREM_REFERENCE_INVALID", "base_authority_v1"
            )
        base_document = dict(base_artifact.json_value)
        v1_validator = AuthorityValidator(self._source_resolver)
        v1_validator.validate(base_document)
        theorem = v1_validator.require_validated_record(
            theorem_record_id,
            RecordKind.CONTEXT_THEOREM_RECORD,
            "application.theorem_record_id",
        )
        resolved_members: list[ResolvedSourceInstance] = []
        for index, member in enumerate(members):
            try:
                resolved_members.append(self._v2_resolver.resolve_member_source_instance(member))
            except (ContextApplicationV2ResolutionError, ResolutionError) as exc:
                raise ContextApplicationV2SemanticValidationError(
                    getattr(exc, "code", None) or "MEMBER_SOURCE_BINDING_MISMATCH",
                    f"members[{index}].source_binding",
                ) from exc

        expected_application_id = ContextApplicationV2InputV1(
            theorem_record_id_bytes=theorem_record_id.digest_bytes,
            members=members,
        ).identity()
        if expected_application_id != application_id:
            raise ContextApplicationV2SemanticValidationError(
                "APPLICATION_IDENTITY_MISMATCH", "application_id"
            )
        if isinstance(application, ContextApplicationV2Record):
            expected_record_id = ContextApplicationV2RecordInputV1(
                context_application_id_bytes=application_id.digest_bytes,
                review_event_ref_v3=application.review_event_ref_v3,
            ).identity()
            if expected_record_id != record_id:
                raise ContextApplicationV2SemanticValidationError(
                    "RECORD_IDENTITY_MISMATCH", "record_id"
                )

        for index, (member, resolved) in enumerate(zip(members, resolved_members, strict=True)):
            self._validate_member(
                member,
                theorem,
                resolved,
                v1_validator,
                f"members[{index}]",
            )
        return ContextApplicationV2ValidationResult(True, len(members))

    def _validate_member(
        self,
        member: object,
        theorem: Mapping[str, object],
        resolved: object,
        v1_validator: object,
        label: str,
    ) -> None:
        from authority_source_resolver import ResolvedSourceInstance
        from authority_validator import AuthorityValidator
        from mtgml.authority import ContextApplicationMemberV2

        if (
            not isinstance(member, ContextApplicationMemberV2)
            or not isinstance(resolved, ResolvedSourceInstance)
            or not isinstance(v1_validator, AuthorityValidator)
        ):
            raise ContextApplicationV2SemanticValidationError("APPLICATION_INPUT_INVALID", label)
        member_wire = member.to_wire()
        member_wire["context_binding"] = _context_binding_to_v1_wire(
            member.context_binding_v1,
            f"{label}.context_binding",
        )
        v1_validator._validate_context_member_source_contract_v1(
            member_wire,
            theorem,
            resolved,
            label,
            evidence_resolver=lambda refs, evidence_label: self._resolve_evidence_refs(
                refs, member, evidence_label
            ),
        )
        source_context = resolved.source_instance_record.get("source_context")
        if not isinstance(source_context, Mapping):
            raise ContextApplicationV2SemanticValidationError(
                "MEMBER_SOURCE_CONTEXT_MISMATCH", f"{label}.source_context"
            )
        source_values = tuple(
            _required_text(source_context.get(name), f"{label}.source_context.{name}")
            for name in CONTEXT_DIMENSIONS
        )
        theorem_context_values = tuple(
            _required_text(value, f"{label}.theorem.context_dimensions[{index}]")
            for index, value in enumerate(_required_array(theorem.get("context_dimensions"), label))
        )
        theorem_temporal_values = tuple(
            _required_text(value, f"{label}.theorem.temporal_semantics[{index}]")
            for index, value in enumerate(_required_array(theorem.get("temporal_semantics"), label))
        )
        bridge = member.context_member_bridge_attestation_v2
        semantic_input = ContextApplicationV2SemanticInput(
            theorem_subject_shape=_context_binding_from_v1_wire(
                theorem.get("subject_shape"), f"{label}.theorem.subject_shape"
            ),
            member_context_binding=cast(PersistenceValue, member.context_binding_v1),
            historical_source_values=source_values,
            bridge_source_values=tuple(slot.source_value for slot in bridge.context),
            theorem_context_values=theorem_context_values,
            bridge_reviewed_values=tuple(slot.reviewed_value for slot in bridge.context),
            bridge_relations=tuple(slot.relation for slot in bridge.context),
            theorem_temporal_values=theorem_temporal_values,
            bridge_temporal_values=tuple(slot.reviewed_value for slot in bridge.temporal),
            theorem_preconditions=_theorem_preconditions(theorem, label),
            member_preconditions=_member_preconditions(member, label),
        )
        validate_context_application_v2_semantics(semantic_input)
        for index, slot in enumerate(bridge.context):
            self._resolve_evidence_refs(slot.evidence_refs, member, f"{label}.context[{index}]")
        for index, slot in enumerate(bridge.temporal):
            self._resolve_evidence_refs(slot.evidence_refs, member, f"{label}.temporal[{index}]")

    def _resolve_evidence_refs(
        self,
        references: Sequence[object],
        member: object,
        label: str,
    ) -> None:
        from authority_source_resolver import ResolutionError
        from context_application_v2_resolver import (
            ContextApplicationV2ResolutionError,
        )
        from mtgml.authority import ContextApplicationMemberV2, EvidenceRefV1

        if not isinstance(member, ContextApplicationMemberV2):
            raise ContextApplicationV2SemanticValidationError("APPLICATION_INPUT_INVALID", label)
        for index, reference in enumerate(references):
            if not isinstance(reference, EvidenceRefV1):
                raise ContextApplicationV2SemanticValidationError(
                    "EVIDENCE_RESOLUTION_FAILURE", f"{label}[{index}]"
                )
            try:
                resolved = self._v2_resolver.resolve_evidence(reference)
            except (ContextApplicationV2ResolutionError, ResolutionError) as exc:
                raise ContextApplicationV2SemanticValidationError(
                    "EVIDENCE_RESOLUTION_FAILURE", f"{label}[{index}]"
                ) from exc
            self._validate_evidence_parent_owner(reference, resolved, member, label)

    def _validate_evidence_parent_owner(
        self,
        reference: object,
        resolved: object,
        member: object,
        label: str,
    ) -> None:
        from context_application_v2_resolver import ResolvedContextEvidence
        from mtgml.authority import ContextApplicationMemberV2, EvidenceRefV1

        if (
            not isinstance(reference, EvidenceRefV1)
            or not isinstance(resolved, ResolvedContextEvidence)
            or not isinstance(member, ContextApplicationMemberV2)
        ):
            raise ContextApplicationV2SemanticValidationError("EVIDENCE_RESOLUTION_FAILURE", label)
        if reference.authority_kind != "c_candidate":
            return
        locator_kind, locator_value = reference.locator
        if reference.path != "sources/m2_5/closures/C/interaction_candidate_universe.v2.json":
            return
        if locator_kind != "json_pointer" or not isinstance(locator_value, str):
            return
        candidate_match = _CANDIDATE_POINTER_ROOT.fullmatch(locator_value)
        source_match = _SOURCE_INSTANCE_POINTER_ROOT.fullmatch(locator_value)
        if candidate_match is None and source_match is None:
            return
        if not isinstance(resolved.artifact.json_value, Mapping):
            raise ContextApplicationV2SemanticValidationError("EVIDENCE_RESOLUTION_FAILURE", label)
        if candidate_match is not None:
            index = int(candidate_match.group(1))
            records = resolved.artifact.json_value.get("candidates")
            parent = self._parent_record(records, index, label)
            self._require_candidate_parent_identity(parent, label)
            if (
                parent.get("candidate_id") != member.candidate_id
                or parent.get("candidate_identity")
                != member.candidate_identity_digest_reference.to_wire()
            ):
                raise ContextApplicationV2SemanticValidationError(
                    "EVIDENCE_SOURCE_SUBSTITUTION", label
                )
            return
        index = int(source_match.group(1))
        records = resolved.artifact.json_value.get("source_instances")
        parent = self._parent_record(records, index, label)
        self._require_source_instance_parent_identity(parent, label)
        if (
            parent.get("source_instance_id") != member.source_instance_id
            or parent.get("candidate_id") != member.candidate_id
        ):
            raise ContextApplicationV2SemanticValidationError("EVIDENCE_SOURCE_SUBSTITUTION", label)

    @staticmethod
    def _require_candidate_parent_identity(parent: Mapping[str, object], label: str) -> None:
        candidate_id = parent.get("candidate_id")
        candidate_identity = parent.get("candidate_identity")
        if not isinstance(candidate_id, str) or not candidate_id:
            raise ContextApplicationV2SemanticValidationError("EVIDENCE_RESOLUTION_FAILURE", label)
        if not isinstance(candidate_identity, Mapping):
            raise ContextApplicationV2SemanticValidationError("EVIDENCE_RESOLUTION_FAILURE", label)
        if set(candidate_identity) != _CANDIDATE_IDENTITY_FIELDS:
            raise ContextApplicationV2SemanticValidationError("EVIDENCE_RESOLUTION_FAILURE", label)
        values = tuple(candidate_identity.values())
        if any(not isinstance(value, str) or not value for value in values):
            raise ContextApplicationV2SemanticValidationError("EVIDENCE_RESOLUTION_FAILURE", label)
        if not _CANDIDATE_DIGEST_HEX.fullmatch(cast(str, candidate_identity["digest_hex"])):
            raise ContextApplicationV2SemanticValidationError("EVIDENCE_RESOLUTION_FAILURE", label)

    @staticmethod
    def _require_source_instance_parent_identity(parent: Mapping[str, object], label: str) -> None:
        for field in ("source_instance_id", "candidate_id"):
            value = parent.get(field)
            if not isinstance(value, str) or not value:
                raise ContextApplicationV2SemanticValidationError(
                    "EVIDENCE_RESOLUTION_FAILURE", label
                )

    @staticmethod
    def _parent_record(value: object, index: int, label: str) -> Mapping[str, object]:
        if not isinstance(value, list) or index < 0 or index >= len(value):
            raise ContextApplicationV2SemanticValidationError("EVIDENCE_RESOLUTION_FAILURE", label)
        parent = value[index]
        if not isinstance(parent, Mapping):
            raise ContextApplicationV2SemanticValidationError("EVIDENCE_RESOLUTION_FAILURE", label)
        return parent


__all__ = [
    "CONTEXT_DIMENSIONS",
    "CONTEXT_SLOT_COUNT",
    "TEMPORAL_SEMANTICS",
    "TEMPORAL_SLOT_COUNT",
    "ContextApplicationV2InformationSensitivityInventory",
    "ContextApplicationV2SemanticInput",
    "ContextApplicationV2SemanticValidationError",
    "ContextApplicationV2SemanticValidationResult",
    "ContextApplicationV2SemanticValidator",
    "ContextPreconditionValueV1",
    "collect_information_sensitivity_facts",
    "validate_context_application_v2_semantics",
]
