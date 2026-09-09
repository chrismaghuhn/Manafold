"""Source-aware semantic validation for the ADR-0045 RPA V2 path."""

from __future__ import annotations

import sys
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from mtgml.authority import (
    RelationApplicationMemberV2,
    RelationApplicationV2,
    SourceBindingDigestV1,
)
from mtgml.persistence import encode_canonical


class RelationApplicationV2SemanticValidationError(ValueError):
    """Stable fail-closed RPA V2 semantic diagnostic."""

    def __init__(self, code: str, location: str, message: str | None = None) -> None:
        self.code = code
        self.location = location
        super().__init__(message or f"{code} at {location}")


@dataclass(frozen=True)
class RelationApplicationV2SemanticValidationResult:
    valid: bool
    divergent_positions: tuple[int, ...]


def _fail(code: str, location: str, message: str | None = None) -> None:
    raise RelationApplicationV2SemanticValidationError(code, location, message)


def _mapping(value: object, label: str) -> Mapping[str, object]:
    if not isinstance(value, Mapping):
        _fail("RELATION_APPLICATION_V2_THEOREM_MISMATCH", label)
    return value


def _array(value: object, label: str, length: int | None = None) -> list[object]:
    if not isinstance(value, list):
        _fail("RELATION_APPLICATION_V2_SOURCE_MISMATCH", label)
    if length is not None and len(value) != length:
        _fail("RELATION_APPLICATION_V2_SOURCE_MISMATCH", label)
    return value


def _text(value: object, code: str, location: str) -> str:
    if not isinstance(value, str) or not value:
        _fail(code, location)
    return value


def _source_instance(resolver: object, member: RelationApplicationMemberV2) -> object:
    binding = member.candidate_universe_binding
    try:
        return resolver.resolve_candidate_source_instance(
            member.candidate_id,
            member.candidate_identity_digest_reference.to_wire(),
            member.source_instance_id,
            SourceBindingDigestV1(
                "candidate_universe",
                cast(str, binding[0]),
                cast(str, binding[1]),
                cast(bytes, binding[2]),
            ),
        )
    except RelationApplicationV2SemanticValidationError:
        raise
    except Exception as exc:
        _fail("RELATION_APPLICATION_V2_SOURCE_MISMATCH", "member.source_instance_id", str(exc))


def _source_participants(resolved: object) -> list[list[object]]:
    record = _mapping(getattr(resolved, "source_instance_record", None), "source instance")
    raw = _array(record.get("participant_bindings"), "source participant bindings")
    result: list[list[object]] = []
    for position, item in enumerate(raw):
        binding = _mapping(item, "source participant binding")
        reference = _mapping(binding.get("participant_ref"), "source participant reference")
        result.append(
            [
                position,
                _text(binding.get("role"), "ROLE_BRIDGE_HISTORICAL_ROLE_MISMATCH", "source role"),
                _text(
                    reference.get("participant_kind"),
                    "ROLE_BRIDGE_PARTICIPANT_MISMATCH",
                    "source participant kind",
                ),
                _text(
                    reference.get("semantic_ref"),
                    "ROLE_BRIDGE_PARTICIPANT_MISMATCH",
                    "source semantic reference",
                ),
            ]
        )
    return result


def _candidate_record(resolved: object) -> Mapping[str, object]:
    candidate = getattr(resolved, "candidate", None)
    return _mapping(getattr(candidate, "candidate_record", None), "candidate record")


def _relation_shape(candidate: Mapping[str, object], participant_count: int) -> tuple[str, str]:
    relation = _text(
        candidate.get("relation"), "RELATION_APPLICATION_V2_SOURCE_MISMATCH", "candidate relation"
    )
    if relation == "declared_card_trigger":
        return "unary", "none"
    if relation == "directional_binary":
        return "binary", "directed"
    if relation == "unordered_binary":
        return "binary", "symmetric"
    if relation == "reviewed_higher_order":
        return "higher_order", "directed"
    if participant_count == 1:
        return "unary", "none"
    return "higher_order", "directed"


def _theorem_subject(theorem: Mapping[str, object]) -> Mapping[str, object]:
    return _mapping(theorem.get("subject"), "relation theorem subject")


def _theorem_participants(theorem: Mapping[str, object]) -> list[list[object]]:
    subject = _theorem_subject(theorem)
    raw = _array(subject.get("participant_roles"), "theorem participant roles")
    result: list[list[object]] = []
    for item in raw:
        record = _mapping(item, "theorem participant role")
        result.append(
            [
                record.get("position"),
                record.get("role"),
                record.get("participant_kind"),
                record.get("semantic_ref"),
            ]
        )
    return result


def _reviewed_binding(member: RelationApplicationMemberV2) -> list[object]:
    fields = _array(member.reviewed_relation_binding_v1, "reviewed relation binding", 5)
    return fields


def _validate_member_proof(
    member: RelationApplicationMemberV2, theorem: Mapping[str, object], label: str
) -> None:
    theorem_kind = _text(
        theorem.get("proof_kind"), "RELATION_APPLICATION_V2_THEOREM_MISMATCH", "proof kind"
    )
    proof = _array(member.member_proof_attestation_v1, "member proof", 2)
    if proof[0] != theorem_kind:
        _fail("RELATION_APPLICATION_V2_THEOREM_MISMATCH", f"{label}.member_proof_attestation")
    theorem_payload = _mapping(theorem.get("proof_payload"), "proof payload")
    if theorem_payload.get("kind") != theorem_kind:
        _fail("RELATION_APPLICATION_V2_THEOREM_MISMATCH", "theorem proof kind")
    member_payload = _array(proof[1], "member proof payload")
    if theorem_kind == "positive_interaction":
        member_payload = _array(member_payload, "positive interaction member proof", 2)
        causal_chain = theorem_payload.get("causal_chain")
        if isinstance(causal_chain, list):
            ordinals = _array(member_payload[0], "causal chain ordinals")
            if ordinals != list(range(len(causal_chain))):
                _fail("CAUSAL_CHAIN_BINDING_MISMATCH", f"{label}.member_proof_attestation")
        template = theorem_payload.get("class_projection_template")
        equivalence = member_payload[1]
        if (template is None) != (equivalence is None):
            _fail("CLASS_PROJECTION_BINDING_MISMATCH", f"{label}.member_proof_attestation")
        if equivalence is not None:
            equivalence_fields = _array(equivalence, "class projection equivalence", 6)
            if encode_canonical(cast(object, equivalence_fields[0])) != encode_canonical(template):
                _fail("CLASS_PROJECTION_BINDING_MISMATCH", f"{label}.member_proof_attestation")
            if encode_canonical(cast(object, equivalence_fields[1])) != encode_canonical(template):
                _fail("CLASS_PROJECTION_BINDING_MISMATCH", f"{label}.member_proof_attestation")
    elif theorem_kind == "positive_separation":
        obligations = _array(
            theorem_payload.get("separation_obligations"), "separation obligations"
        )
        coverages = member_payload
        if len(obligations) != len(coverages):
            _fail("SEPARATION_COVERAGE_INCOMPLETE", f"{label}.member_proof_attestation")
        for obligation, coverage in zip(obligations, coverages, strict=True):
            obligation_record = _mapping(obligation, "separation obligation")
            coverage_fields = _array(coverage, "separation coverage", 6)
            if coverage_fields[0] != obligation_record.get("channel") or coverage_fields[
                1
            ] != obligation_record.get("required_conclusion"):
                _fail("SEPARATION_COVERAGE_MISMATCH", f"{label}.member_proof_attestation")
    elif theorem_kind == "model_bound_scope":
        expected_scope = _mapping(theorem_payload, "scope proof payload")
        scope_fields = _array(member_payload, "scope member proof", 1)
        actual_scope = _array(scope_fields[0], "scope boundary attestation", 6)
        for field in (
            (0, "model_id"),
            (1, "model_version"),
            (3, "reason_code"),
            (4, "observed_candidate_shape"),
            (2, "model_boundary_ref"),
        ):
            index, name = field
            expected_value = expected_scope.get(name)
            if expected_value is not None and actual_scope[index] != expected_value:
                _fail("SCOPE_BINDING_MISMATCH", f"{label}.member_proof_attestation")


def _validate_preconditions(
    member: RelationApplicationMemberV2,
    theorem: Mapping[str, object],
    resolved: object,
    source_resolver: object,
    relation_binding: list[object],
    label: str,
) -> None:
    theorem_preconditions = _array(theorem.get("preconditions", []), "theorem preconditions")
    attestations = _array(member.precondition_attestations_v1, "member precondition attestations")
    if len(theorem_preconditions) != len(attestations):
        _fail("PRECONDITION_COVERAGE", label)
    source = _mapping(getattr(resolved, "source_instance_record", None), "source instance")
    source_context = _mapping(source.get("source_context", {}), "source context")
    source_participants = _source_participants(resolved)
    for index, (expected_raw, attestation_raw) in enumerate(
        zip(theorem_preconditions, attestations, strict=True)
    ):
        expected = _mapping(expected_raw, "theorem precondition")
        attestation = _array(attestation_raw, "precondition attestation", 4)
        if attestation[0] != expected.get("precondition_id"):
            _fail("PRECONDITION_MISMATCH", f"{label}.precondition_attestations[{index}]")
        expected_value = expected.get("payload")
        if attestation[1] != expected_value:
            _fail("PRECONDITION_MISMATCH", f"{label}.precondition_attestations[{index}")
        kind = _text(
            expected.get("precondition_kind"), "PRECONDITION_MISMATCH", "precondition kind"
        )
        if kind == "candidate_relation_shape":
            subject = _theorem_subject(theorem)
            candidate = _candidate_record(resolved)
            arity, directionality = _relation_shape(candidate, len(source_participants))
            observed = [
                candidate.get("scope"),
                candidate.get("relation"),
                directionality,
                relation_binding[3],
            ]
            expected_shape = [
                subject.get("scope", candidate.get("scope")),
                subject.get("relation"),
                subject.get("directionality"),
                subject.get("host_relationship"),
            ]
            if observed != expected_shape or arity != subject.get("arity"):
                _fail("PRECONDITION_SOURCE_MISMATCH", f"{label}.precondition_attestations[{index}]")
        elif kind == "participant_binding":
            expected_binding = _array(expected_value, "participant binding", 4)
            position = expected_binding[0]
            matches = [item for item in source_participants if item[0] == position]
            if len(matches) != 1 or matches[0] != expected_binding:
                _fail("PRECONDITION_SOURCE_MISMATCH", f"{label}.precondition_attestations[{index}")
        elif kind == "source_context":
            context = _array(expected_value, "source context precondition", 2)
            if (
                context[0] not in source_context
                or [context[0], source_context[context[0]]] != context
            ):
                _fail("PRECONDITION_SOURCE_MISMATCH", f"{label}.precondition_attestations[{index}")
        elif kind == "class_projection":
            _validate_member_proof(member, theorem, label)
        elif kind == "b2_boundary":
            resolver = getattr(source_resolver, "resolve_relation_application_v2_b2_boundary", None)
            if not callable(resolver):
                _fail(
                    "B2_PRECONDITION_RESOLUTION_REQUIRED",
                    f"{label}.precondition_attestations[{index}",
                )
            try:
                resolver(expected_value)
            except Exception as exc:
                _fail(
                    "B2_PRECONDITION_SOURCE_MISMATCH",
                    f"{label}.precondition_attestations[{index}",
                    str(exc),
                )
        elif kind == "temporal_semantic":
            continue
        else:
            _fail("PRECONDITION_KIND_UNSUPPORTED", f"{label}.precondition_attestations[{index}")


def validate_relation_application_v2_semantics(
    application: RelationApplicationV2 | RelationApplicationMemberV2,
    source_resolver: object,
    *,
    theorem_record: Mapping[str, object],
) -> RelationApplicationV2SemanticValidationResult:
    """Validate RPA V2 semantics while preserving every V1 source fact."""

    members = (
        application.members if isinstance(application, RelationApplicationV2) else (application,)
    )
    if not members:
        _fail("RELATION_APPLICATION_V2_SOURCE_MISMATCH", "members")
    expected_theorem = _theorem_participants(theorem_record)
    subject = _theorem_subject(theorem_record)
    proof_kind = _text(
        theorem_record.get("proof_kind"), "RELATION_APPLICATION_V2_THEOREM_MISMATCH", "proof_kind"
    )
    expected_disposition = {
        "positive_interaction": "required_interaction",
        "positive_separation": "not_an_interaction_with_proof",
        "model_bound_scope": "out_of_declared_scope_with_reason",
    }.get(proof_kind)
    if (
        isinstance(application, RelationApplicationV2)
        and application.terminal_disposition != expected_disposition
    ):
        _fail("RELATION_APPLICATION_V2_THEOREM_MISMATCH", "terminal_disposition")
    divergent_positions: set[int] = set()
    has_exact_member = False
    has_divergent_member = False
    for member_index, member in enumerate(members):
        label = f"members[{member_index}]"
        resolved = _source_instance(source_resolver, member)
        source_participants = _source_participants(resolved)
        candidate = _candidate_record(resolved)
        relation_binding = _reviewed_binding(member)
        reviewed_participants = _array(relation_binding[4], "reviewed participant bindings")
        if len(source_participants) != len(expected_theorem) or len(reviewed_participants) != len(
            expected_theorem
        ):
            _fail("ROLE_BRIDGE_PARTICIPANT_MISMATCH", label)
        if relation_binding[:4] != [
            subject.get("scope", candidate.get("scope")),
            subject.get("relation"),
            subject.get("directionality"),
            subject.get("host_relationship"),
        ]:
            _fail("RELATION_APPLICATION_V2_THEOREM_MISMATCH", f"{label}.relation_binding")
        if relation_binding[0] != candidate.get("scope") or relation_binding[1] != candidate.get(
            "relation"
        ):
            _fail("RELATION_APPLICATION_V2_SOURCE_MISMATCH", f"{label}.relation_binding")
        if [[item[2], item[3]] for item in reviewed_participants] != [
            [item[2], item[3]] for item in source_participants
        ]:
            _fail("ROLE_BRIDGE_PARTICIPANT_MISMATCH", f"{label}.relation_binding")
        if [list(item) for item in reviewed_participants] != expected_theorem:
            _fail("ROLE_BRIDGE_REVIEWED_ROLE_MISMATCH", f"{label}.relation_binding")

        entries = member.participant_role_bridge_v1.entries
        if len(entries) != len(source_participants) or len(entries) != len(expected_theorem):
            _fail("ROLE_BRIDGE_MISSING", f"{label}.participant_role_bridge")
        member_divergent = False
        for entry, source, reviewed in zip(
            entries, source_participants, reviewed_participants, strict=True
        ):
            if entry.position != source[0] or entry.position != reviewed[0]:
                _fail("ROLE_BRIDGE_POSITION_MISMATCH", f"{label}.participant_role_bridge")
            if entry.participant_kind != source[2] or entry.semantic_ref != source[3]:
                _fail("ROLE_BRIDGE_PARTICIPANT_MISMATCH", f"{label}.participant_role_bridge")
            if entry.historical_source_role != source[1]:
                _fail("ROLE_BRIDGE_HISTORICAL_ROLE_MISMATCH", f"{label}.participant_role_bridge")
            if entry.reviewed_role != reviewed[1]:
                _fail("ROLE_BRIDGE_REVIEWED_ROLE_MISMATCH", f"{label}.participant_role_bridge")
            if entry.historical_source_role == entry.reviewed_role:
                continue
            else:
                member_divergent = True
                divergent_positions.add(entry.position)
        if member_divergent:
            has_divergent_member = True
        else:
            has_exact_member = True
        _validate_member_proof(member, theorem_record, label)
        _validate_preconditions(
            member,
            theorem_record,
            resolved,
            source_resolver,
            relation_binding,
            label,
        )

    if has_exact_member:
        _fail("RELATION_APPLICATION_V2_NOT_DIVERGENT", "members")
    if not has_divergent_member:
        _fail("RELATION_APPLICATION_V2_NOT_DIVERGENT", "members")
    return RelationApplicationV2SemanticValidationResult(True, tuple(sorted(divergent_positions)))


__all__ = [
    "RelationApplicationV2SemanticValidationError",
    "RelationApplicationV2SemanticValidationResult",
    "validate_relation_application_v2_semantics",
]
