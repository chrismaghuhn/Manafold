"""Semantic validation for ADR 0045 ContextApplicationV3."""

from __future__ import annotations

import copy
import sys
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from context_application_v2_validator import (
    CONTEXT_DIMENSIONS,
    ContextApplicationV2SemanticInput,
    ContextPreconditionValueV1,
    validate_context_application_v2_semantics,
)
from context_application_v3_resolver import (
    ContextApplicationV3ResolutionError,
    ContextApplicationV3Resolver,
    ResolvedRpaV2Member,
)
from mtgml.authority import (
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationMemberV3,
    ContextApplicationV3InputV1,
    ContextApplicationV3Record,
)


class ContextApplicationV3SemanticValidationError(ValueError):
    def __init__(self, code: str, location: str, *, cause_code: str | None = None) -> None:
        self.code = code
        self.location = location
        self.cause_code = cause_code
        super().__init__(f"{code} at {location}")


@dataclass(frozen=True)
class ContextApplicationV3SemanticValidationResult:
    valid: bool
    member_count: int


def _preconditions(value: object, label: str) -> tuple[ContextPreconditionValueV1, ...]:
    if not isinstance(value, list):
        raise ContextApplicationV3SemanticValidationError("PRECONDITION_MISMATCH", label)
    result: list[ContextPreconditionValueV1] = []
    for index, item in enumerate(value):
        if not isinstance(item, list) or len(item) != 4 or not isinstance(item[0], str):
            raise ContextApplicationV3SemanticValidationError(
                "PRECONDITION_MISMATCH", f"{label}[{index}]"
            )
        result.append(ContextPreconditionValueV1(item[0], cast(object, item[1])))
    return tuple(result)


def _context_binding_from_wire(value: object, label: str) -> list[object]:
    if not isinstance(value, Mapping) or set(value) != {
        "arity",
        "directionality",
        "participant_roles",
        "host_relationship",
    }:
        raise ContextApplicationV3SemanticValidationError(
            "CONTEXT_APPLICATION_V3_MEMBER_MISMATCH", label
        )
    participants = value["participant_roles"]
    if not isinstance(participants, list):
        raise ContextApplicationV3SemanticValidationError(
            "CONTEXT_APPLICATION_V3_MEMBER_MISMATCH", label
        )
    result: list[list[object]] = []
    for index, item in enumerate(participants):
        if not isinstance(item, Mapping) or set(item) != {
            "position",
            "role",
            "participant_kind",
            "semantic_ref",
        }:
            raise ContextApplicationV3SemanticValidationError(
                "CONTEXT_APPLICATION_V3_MEMBER_MISMATCH", f"{label}.participant_roles[{index}]"
            )
        result.append(
            [item["position"], item["role"], item["participant_kind"], item["semantic_ref"]]
        )
    return [value["arity"], value["directionality"], result, value["host_relationship"]]


def _source_participants(resolved: object, label: str) -> list[list[object]]:
    record = getattr(resolved, "source_instance_record", None)
    if not isinstance(record, Mapping) or not isinstance(record.get("participant_bindings"), list):
        raise ContextApplicationV3SemanticValidationError(
            "CONTEXT_APPLICATION_V3_SOURCE_MISMATCH", f"{label}.source_instance"
        )
    result: list[list[object]] = []
    for position, item in enumerate(record["participant_bindings"]):
        if not isinstance(item, Mapping) or not isinstance(item.get("participant_ref"), Mapping):
            raise ContextApplicationV3SemanticValidationError(
                "CONTEXT_APPLICATION_V3_SOURCE_MISMATCH", f"{label}.source_instance.participants"
            )
        ref = item["participant_ref"]
        result.append(
            [position, item.get("role"), ref.get("participant_kind"), ref.get("semantic_ref")]
        )
    return result


def _theorem_values(theorem: Mapping[str, object], key: str, label: str) -> tuple[str, ...]:
    value = theorem.get(key)
    if not isinstance(value, list) or any(not isinstance(item, str) for item in value):
        raise ContextApplicationV3SemanticValidationError(
            "CONTEXT_APPLICATION_V3_MEMBER_MISMATCH", label
        )
    return tuple(value)


def _rpa_role_equations(
    member: ContextApplicationMemberV3,
    resolved: object,
    theorem: Mapping[str, object],
    rpa: ResolvedRpaV2Member,
    label: str,
) -> bool:
    source = _source_participants(resolved, label)
    context_binding = member.reviewed_context_binding_v1
    theorem_binding = _context_binding_from_wire(
        {
            "arity": theorem.get("subject_shape", {}).get("arity"),
            "directionality": theorem.get("subject_shape", {}).get("directionality"),
            "participant_roles": theorem.get("subject_shape", {}).get("participant_roles"),
            "host_relationship": theorem.get("subject_shape", {}).get("host_relationship"),
        },
        f"{label}.theorem.subject_shape",
    )
    if context_binding != theorem_binding:
        raise ContextApplicationV3SemanticValidationError(
            "CONTEXT_APPLICATION_V3_MEMBER_MISMATCH", f"{label}.context_binding"
        )
    rpa_wire = rpa.member.to_wire()
    relation = cast(Mapping[str, object], rpa_wire["relation_binding"])
    bridge = cast(list[object], rpa_wire["participant_role_bridge"])
    reviewed = cast(list[object], relation["participant_bindings"])
    if len(source) != len(bridge) or len(source) != len(reviewed):
        raise ContextApplicationV3SemanticValidationError(
            "CONTEXT_APPLICATION_V3_ROLE_MISMATCH", f"{label}.participants"
        )
    divergent = False
    for index, (source_item, bridge_item, relation_item, theorem_item) in enumerate(
        zip(source, bridge, reviewed, theorem_binding[2], strict=True)
    ):
        if not isinstance(bridge_item, Mapping) or not isinstance(relation_item, Mapping):
            raise ContextApplicationV3SemanticValidationError(
                "CONTEXT_APPLICATION_V3_ROLE_MISMATCH", f"{label}.participants[{index}]"
            )
        historical = [
            bridge_item.get("position"),
            bridge_item.get("historical_source_role"),
            bridge_item.get("participant_kind"),
            bridge_item.get("semantic_ref"),
        ]
        reviewed_item = [
            relation_item.get("position"),
            relation_item.get("role"),
            relation_item.get("participant_kind"),
            relation_item.get("semantic_ref"),
        ]
        if source_item != historical or theorem_item != reviewed_item:
            raise ContextApplicationV3SemanticValidationError(
                "CONTEXT_APPLICATION_V3_ROLE_MISMATCH", f"{label}.participants[{index}]"
            )
        divergent = divergent or historical[1] != reviewed_item[1]
    if not divergent:
        raise ContextApplicationV3SemanticValidationError(
            "CONTEXT_APPLICATION_V3_NOT_DIVERGENT", label
        )
    return divergent


class ContextApplicationV3SemanticValidator:
    def __init__(self, resolver: ContextApplicationV3Resolver) -> None:
        self._resolver = resolver

    def validate(
        self, application: ContextApplicationV3Record | ContextApplicationV3InputV1
    ) -> ContextApplicationV3SemanticValidationResult:
        if not isinstance(application, ContextApplicationV3Record | ContextApplicationV3InputV1):
            raise ContextApplicationV3SemanticValidationError(
                "CONTEXT_APPLICATION_V3_INPUT_INVALID", "application"
            )
        theorem_id = (
            application.theorem_record_id
            if isinstance(application, ContextApplicationV3Record)
            else AuthorityIdentityV1(
                AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
                application.theorem_record_id_bytes,
            )
        )
        if isinstance(application, ContextApplicationV3Record):
            expected_record = application.__class__.from_parts(
                application.application_id,
                application.theorem_record_id,
                application.members,
                application.review_event_ref_v4,
            )
            if expected_record.record_id != application.record_id:
                raise ContextApplicationV3SemanticValidationError(
                    "CONTEXT_APPLICATION_V3_IDENTITY_MISMATCH", "record_id"
                )
            application_id = application.application_id
        else:
            application_id = application.identity()
        theorem = self._resolver.resolve_current_context_theorem(theorem_id)
        for index, member in enumerate(application.members):
            label = f"members[{index}]"
            try:
                resolved = self._resolver.resolve_member_source_instance(member)
                rpa = self._resolver._rpa_member_resolver.resolve_current_rpa_member(
                    AuthorityIdentityV1(
                        AuthorityIdentityKind.RELATION_APPLICATION_V2,
                        member.relation_application_v2_id_bytes,
                    ),
                    member.candidate_id,
                    member.candidate_identity_digest_reference,
                    member.source_instance_id,
                )
                _rpa_role_equations(member, resolved, theorem, rpa, label)
                self._validate_v1_source_contract(member, theorem, resolved, rpa, label)
            except ContextApplicationV3SemanticValidationError:
                raise
            except ContextApplicationV3ResolutionError as exc:
                raise ContextApplicationV3SemanticValidationError(
                    exc.code, exc.location, cause_code=exc.cause_code
                ) from exc
            except Exception as exc:
                raise ContextApplicationV3SemanticValidationError(
                    "CONTEXT_APPLICATION_V3_SOURCE_MISMATCH", label, cause_code=type(exc).__name__
                ) from exc
        if (
            isinstance(application, ContextApplicationV3Record)
            and application.application_id != application_id
        ):
            raise ContextApplicationV3SemanticValidationError(
                "CONTEXT_APPLICATION_V3_IDENTITY_MISMATCH", "application_id"
            )
        return ContextApplicationV3SemanticValidationResult(True, len(application.members))

    def _validate_v1_source_contract(
        self,
        member: ContextApplicationMemberV3,
        theorem: Mapping[str, object],
        resolved: object,
        rpa: ResolvedRpaV2Member,
        label: str,
    ) -> None:
        validator, _ = self._resolver._validated_base()
        source_participants = _source_participants(resolved, label)
        member_wire = member.to_wire()
        historical_binding = copy.deepcopy(cast(dict[str, object], member_wire["context_binding"]))
        historical_binding["participant_roles"] = [
            {
                "position": item[0],
                "role": item[1],
                "participant_kind": item[2],
                "semantic_ref": item[3],
            }
            for item in source_participants
        ]
        member_wire["context_binding"] = historical_binding
        theorem_for_source = copy.deepcopy(dict(theorem))
        subject_shape = copy.deepcopy(cast(dict[str, object], theorem["subject_shape"]))
        subject_shape["participant_roles"] = historical_binding["participant_roles"]
        theorem_for_source["subject_shape"] = subject_shape
        rpa_proof = rpa.member.to_wire().get("member_proof_attestation")
        if rpa_proof is not None:
            member_wire["member_proof_attestation"] = rpa_proof
        validator._validate_context_member_source_contract_v1(
            member_wire,
            theorem_for_source,
            resolved,
            label,
        )
        theorem_context = _theorem_values(theorem, "context_dimensions", f"{label}.context")
        theorem_temporal = _theorem_values(theorem, "temporal_semantics", f"{label}.temporal")
        bridge = member.context_member_bridge_attestation_v2
        source_context = resolved.source_instance_record.get("source_context")
        if not isinstance(source_context, Mapping):
            raise ContextApplicationV3SemanticValidationError(
                "CONTEXT_APPLICATION_V3_SOURCE_MISMATCH", f"{label}.source_context"
            )
        historical_values = tuple(source_context[name] for name in CONTEXT_DIMENSIONS)
        semantic = ContextApplicationV2SemanticInput(
            theorem_subject_shape=cast(
                object, _context_binding_from_wire(theorem["subject_shape"], label)
            ),
            member_context_binding=cast(object, member.reviewed_context_binding_v1),
            historical_source_values=cast(tuple[str, ...], historical_values),
            bridge_source_values=tuple(slot.source_value for slot in bridge.context),
            theorem_context_values=theorem_context,
            bridge_reviewed_values=tuple(slot.reviewed_value for slot in bridge.context),
            bridge_relations=tuple(slot.relation for slot in bridge.context),
            theorem_temporal_values=theorem_temporal,
            bridge_temporal_values=tuple(slot.reviewed_value for slot in bridge.temporal),
            theorem_preconditions=_preconditions(theorem.get("preconditions"), label),
            member_preconditions=_preconditions(member.precondition_attestations_v1, label),
        )
        validate_context_application_v2_semantics(semantic)


__all__ = [
    "ContextApplicationV3SemanticValidationError",
    "ContextApplicationV3SemanticValidationResult",
    "ContextApplicationV3SemanticValidator",
]
