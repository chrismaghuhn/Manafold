"""Build one non-authoritative, source-bound M2.5.C review packet.

The packet is a portable input to a human review.  It never creates an
acceptance event, an authority record, a semantic conclusion, or a C
classification.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import sys
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Final, NoReturn, TypeAlias, cast

from authority_source_resolver import (
    B1_FINAL_AUTHORITY_IDS,
    B1_FINAL_CITATIONS_SCHEMA,
    B1_FINAL_CLOSURE_SCHEMA,
    B2_CATALOG_PATH,
    B2_CATALOG_SCHEMA,
    B2_CLASSIFICATION_PATH,
    B2_CLASSIFICATION_SCHEMA,
    B2_CLOSURE_PATH,
    B2_CLOSURE_SCHEMA,
    REV3_SOURCE_COLUMNS,
    SOURCE_CONTEXT_KEYS,
    AuthoritySourceResolver,
    B1FinalArtifactBindingsV1,
    B2ArtifactBindingsV1,
    B2BoundaryReferenceV1,
    ResolutionError,
    ResolutionStatus,
    ResolvedSourceInstance,
    _b1_validate_header,
    _b1_verify_closure_binding,
    _parse_rev3_rows,
)
from build_m2_5_c_authority_review_worklist import (
    WORKLIST_FORMAT,
    LoadedReviewInputs,
    _json_bytes,
    _plain,
    _work_item,
    build_worklist,
    load_review_inputs,
)
from build_m2_5_c_authority_review_worklist import (
    _manifest as _worklist_manifest,
)
from mtgml.authority import SourceBindingDigestV1

ROOT = Path(__file__).resolve().parents[1]
CANARY_ORDINAL: Final = 0
PACKET_FORMAT: Final = "manafold.m2.5.c.canary-review-packet.v1"
CHECKLIST_ID: Final = "interaction-authority-review-checklist.v1"
READY_FOR_HUMAN_REVIEW: Final = "READY_FOR_HUMAN_REVIEW"
BLOCKED: Final = "BLOCKED"
FAIL: Final = "FAIL"
PACKET_DIRECTORY_NAME: Final = "canary"
PACKET_FILES: Final = (
    "manifest.v1.json",
    "source_inventory.v1.json",
    "review_worksheet.v1.json",
)
ALL_PACKET_FILES: Final = (*PACKET_FILES, "REVIEW_PACKET.md")
AWAITING_HUMAN_REVIEW: Final = "AWAITING_HUMAN_REVIEW"
SOURCE_FACT: Final = "SOURCE_FACT"
REVIEWER_DECISION: Final = "REVIEWER_DECISION"
_FORBIDDEN_KEYS: Final = frozenset(
    {
        "human_accepted",
        "review_event_ref",
        "accepted_theorem_record",
        "accepted_application_record",
        "terminal_disposition",
        "interaction_class_id",
        "semantic_class",
    }
)
_FORBIDDEN_VALUES: Final = frozenset(
    {
        "human_accepted",
        "required_interaction",
        "not_an_interaction_with_proof",
        "out_of_declared_scope_with_reason",
        "positive_interaction",
        "positive_separation",
        "model_bound_scope",
    }
)

JsonObject: TypeAlias = dict[str, object]


class CanaryPacketError(ValueError):
    """A fail-closed packet build or qualification error."""

    def __init__(self, code: str, message: str, status: str = FAIL) -> None:
        self.code = code
        self.message = message
        self.status = status
        super().__init__(f"[{status}:{code}] {message}")


@dataclass(frozen=True)
class CanaryPacketResult:
    """Immutable result of one packet build."""

    status: str
    packet_dir: Path
    worklist_path: Path
    worklist_sha256: str
    packet_sha256: str


def _fail(code: str, message: str) -> NoReturn:
    raise CanaryPacketError(code, message)


def _object(value: object, label: str) -> Mapping[str, object]:
    if not isinstance(value, Mapping):
        _fail("PACKET_SHAPE_INVALID", f"{label} must be an object")
    return cast(Mapping[str, object], value)


def _array(value: object, label: str) -> list[object]:
    if not isinstance(value, list):
        _fail("PACKET_SHAPE_INVALID", f"{label} must be an array")
    return cast(list[object], value)


def _text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        _fail("PACKET_VALUE_INVALID", f"{label} must be non-empty text")
    return value


def _digest(value: object, label: str) -> str:
    digest = _text(value, label)
    if len(digest) != 64 or any(char not in "0123456789abcdef" for char in digest):
        _fail("PACKET_VALUE_INVALID", f"{label} must be lowercase SHA-256 hex")
    return digest


def _binding_from_record(
    value: object,
    role: str,
    expected_path: str,
    expected_schema: str,
    label: str,
) -> SourceBindingDigestV1:
    record = _object(value, label)
    if set(record) != {"path", "raw_sha256"}:
        _fail("SOURCE_BINDING_INVALID", f"{label} fields are not closed")
    path = _text(record.get("path"), f"{label}.path")
    if path != expected_path:
        _fail("SOURCE_BINDING_INVALID", f"{label}.path is not the accepted path")
    return SourceBindingDigestV1(
        artifact_role=role,
        path=path,
        schema_or_null=expected_schema,
        raw_sha256=bytes.fromhex(_digest(record.get("raw_sha256"), f"{label}.raw_sha256")),
    )


def _binding_from_manifest(value: object, label: str) -> SourceBindingDigestV1:
    record = _object(value, label)
    if set(record) != {"path", "schema", "raw_sha256"}:
        _fail("SOURCE_BINDING_INVALID", f"{label} fields are not closed")
    return SourceBindingDigestV1(
        artifact_role="candidate_universe",
        path=_text(record.get("path"), f"{label}.path"),
        schema_or_null=_text(record.get("schema"), f"{label}.schema"),
        raw_sha256=bytes.fromhex(_digest(record.get("raw_sha256"), f"{label}.raw_sha256")),
    )


def _binding_json(binding: SourceBindingDigestV1) -> JsonObject:
    return {
        "artifact_role": binding.artifact_role,
        "path": binding.path,
        "schema_or_null": binding.schema_or_null,
        "raw_sha256": binding.raw_sha256.hex(),
    }


def _b2_bindings(inputs: LoadedReviewInputs) -> B2ArtifactBindingsV1:
    raw_inputs = _object(
        inputs.candidate_universe.get("input_bindings"), "candidate input bindings"
    )
    raw_artifacts = _array(raw_inputs.get("b2_artifacts"), "candidate B2 input bindings")
    if len(raw_artifacts) != 3:
        _fail("SOURCE_BINDING_COVERAGE", "candidate B2 input bindings must contain three artifacts")
    return B2ArtifactBindingsV1(
        catalog=_binding_from_record(
            raw_artifacts[0], "b2_catalog", B2_CATALOG_PATH, B2_CATALOG_SCHEMA, "B2 catalog binding"
        ),
        classifications=_binding_from_record(
            raw_artifacts[1],
            "b2_classifications",
            B2_CLASSIFICATION_PATH,
            B2_CLASSIFICATION_SCHEMA,
            "B2 classification binding",
        ),
        closure=_binding_from_record(
            raw_artifacts[2], "b2_closure", B2_CLOSURE_PATH, B2_CLOSURE_SCHEMA, "B2 closure binding"
        ),
    )


def _b1_bindings(inputs: LoadedReviewInputs) -> B1FinalArtifactBindingsV1:
    raw_inputs = _object(
        inputs.candidate_universe.get("input_bindings"), "candidate input bindings"
    )
    raw_artifacts = _array(raw_inputs.get("b1_final_artifacts"), "candidate B1 input bindings")
    if len(raw_artifacts) != 2:
        _fail("SOURCE_BINDING_COVERAGE", "candidate B1 input bindings must contain two artifacts")
    return B1FinalArtifactBindingsV1(
        citations=_binding_from_record(
            raw_artifacts[0],
            "b1_final_citations",
            "sources/m2_5/closures/B1/official_authority_citations.v3.json",
            B1_FINAL_CITATIONS_SCHEMA,
            "B1.Final citations binding",
        ),
        closure=_binding_from_record(
            raw_artifacts[1],
            "b1_final_closure",
            "sources/m2_5/closures/B1/official_authority_citation_closure.v2.json",
            B1_FINAL_CLOSURE_SCHEMA,
            "B1.Final closure binding",
        ),
    )


def _read_worklist_canary(
    worklist_path: Path, inputs: LoadedReviewInputs
) -> tuple[JsonObject, JsonObject, str]:
    try:
        raw = worklist_path.read_bytes()
        lines = raw.splitlines()
        manifest = _object(json.loads(lines[0]), "worklist manifest")
        item = _object(json.loads(lines[CANARY_ORDINAL + 1]), "worklist canary")
    except (OSError, IndexError, json.JSONDecodeError) as exc:
        _fail("WORKLIST_INVALID", f"cannot read the generated worklist: {exc}")
    if manifest.get("format") != "manafold.m2.5.c.authority-review-worklist.v1":
        _fail("WORKLIST_BINDING_MISMATCH", "worklist format is not V1")
    if manifest.get("source_commit") != inputs.source_commit:
        _fail("WORKLIST_BINDING_MISMATCH", "worklist source commit differs from loaded sources")
    if item.get("ordinal") != CANARY_ORDINAL:
        _fail("CANARY_SELECTION_INVALID", "worklist ordinal zero is not present")
    candidate_id = _text(item.get("candidate_id"), "canary candidate ID")
    classification = inputs.classification_records[CANARY_ORDINAL]
    if candidate_id != classification.get("candidate_id"):
        _fail("CANARY_SELECTION_INVALID", "ordinal zero does not match the classification order")
    candidate = next(
        (
            record
            for record in inputs.candidate_records
            if record.get("candidate_id") == candidate_id
        ),
        None,
    )
    instance = next(
        (
            record
            for record in inputs.source_instance_records
            if record.get("candidate_id") == candidate_id
        ),
        None,
    )
    if candidate is None or instance is None:
        _fail("CANARY_BINDING_MISMATCH", "ordinal-zero candidate lacks an exact source record")
    if item.get("candidate_identity") != candidate.get("candidate_identity"):
        _fail("CANARY_BINDING_MISMATCH", "worklist candidate identity differs from the C ledger")
    if item.get("source_instance_id") != instance.get("source_instance_id"):
        _fail("CANARY_BINDING_MISMATCH", "worklist source instance differs from the C ledger")
    if item.get("candidate_source_binding") != candidate.get("source_binding"):
        _fail("CANARY_BINDING_MISMATCH", "worklist candidate binding differs from the C ledger")
    if item.get("source_instance_binding") != instance.get("source_binding"):
        _fail("CANARY_BINDING_MISMATCH", "worklist instance binding differs from the C ledger")
    return dict(manifest), dict(item), hashlib.sha256(raw).hexdigest()


def _expected_worklist_bytes(inputs: LoadedReviewInputs) -> bytes:
    candidates = {
        _text(record.get("candidate_id"), "candidate ID"): record
        for record in inputs.candidate_records
    }
    instances = {
        _text(record.get("candidate_id"), "source-instance candidate ID"): record
        for record in inputs.source_instance_records
    }
    lines = [_json_bytes(_worklist_manifest(inputs))]
    for ordinal, classification in enumerate(inputs.classification_records):
        candidate_id = _text(classification.get("candidate_id"), "classification candidate ID")
        lines.append(
            _json_bytes(
                _work_item(
                    ordinal,
                    candidates[candidate_id],
                    instances[candidate_id],
                    classification,
                    inputs.review_domains,
                )
            )
        )
    return b"".join(lines)


def _classification_fact(classification: Mapping[str, object]) -> JsonObject:
    allowed = (
        "candidate_id",
        "evidence_refs",
        "reconciliation",
        "review_domain_assessments",
        "review_rationale",
        "review_state",
        "source_instance_context_mappings",
        "unresolved_reason",
    )
    return {key: _plain(classification[key]) for key in allowed}


def _resolve_rev3(
    resolver: AuthoritySourceResolver,
    inputs: LoadedReviewInputs,
    candidate: Mapping[str, object],
    instance: Mapping[str, object],
) -> tuple[ResolvedSourceInstance | None, JsonObject]:
    binding = _binding_from_manifest(
        inputs.candidate_universe_binding, "candidate universe binding"
    )
    try:
        resolved = resolver.resolve_candidate_source_instance(
            _text(candidate.get("candidate_id"), "candidate ID"),
            _object(candidate.get("candidate_identity"), "candidate identity"),
            _text(instance.get("source_instance_id"), "source instance ID"),
            binding,
        )
    except ResolutionError as exc:
        if exc.status is ResolutionStatus.BLOCKED:
            return None, {
                "kind": SOURCE_FACT,
                "status": BLOCKED,
                "reason_code": exc.code,
                "detail": (
                    "required external REV3 bytes are unavailable; "
                    "the C projection is not substituted"
                ),
                "required_binding": _plain(instance.get("source_binding")),
            }
        raise CanaryPacketError(exc.code, exc.message, exc.status.value) from exc
    if not isinstance(resolved, ResolvedSourceInstance):
        _fail("REV3_RESOLUTION_INVALID", "source resolver returned an unverified canary result")
    return resolved, _rev3_fact(resolved, candidate, instance)


def _rev3_fact(
    resolved: ResolvedSourceInstance,
    candidate: Mapping[str, object],
    instance: Mapping[str, object],
) -> JsonObject:
    source_artifact = resolved.source_artifact
    row_ordinal = cast(int, cast(Mapping[str, object], instance["source_binding"])["row_ordinal"])
    rows = _parse_rev3_rows(source_artifact.raw_bytes, source_artifact.path)
    if row_ordinal < 0 or row_ordinal >= len(rows):
        _fail(
            "REV3_ROW_BINDING_MISMATCH", "resolved canary row ordinal is outside the verified CSV"
        )
    values = rows[row_ordinal]
    expected_values = cast(
        list[str], cast(Mapping[str, object], instance["source_binding"])["source_values"]
    )
    if values != expected_values:
        _fail(
            "REV3_ROW_BINDING_MISMATCH", "resolved canary row differs from source-instance values"
        )
    return {
        "kind": SOURCE_FACT,
        "status": "PASS",
        "archive_member": source_artifact.path,
        "archive_member_sha256": source_artifact.raw_sha256,
        "row_ordinal": row_ordinal,
        "locator": {
            "kind": "archive_member",
            "value": source_artifact.path,
            "row_ordinal": row_ordinal,
        },
        "source_columns": list(REV3_SOURCE_COLUMNS),
        "source_values": values,
        "mechanical_candidate_projection": {
            "scope": candidate["scope"],
            "relation": candidate["relation"],
            "participant_refs": _plain(candidate["participant_refs"]),
            "supporting_requirement_ids": _plain(candidate["supporting_requirement_ids"]),
        },
    }


def _resolve_b2_inventory(
    resolver: AuthoritySourceResolver,
    inputs: LoadedReviewInputs,
    candidate: Mapping[str, object],
    classification: Mapping[str, object],
) -> JsonObject:
    bindings = _b2_bindings(inputs)
    family_ids: list[str] = []
    for index, raw_ref in enumerate(
        _array(candidate.get("participant_refs"), "candidate participants")
    ):
        ref = _object(raw_ref, f"candidate participant[{index}]")
        if ref.get("participant_kind") == "requirement_family":
            family_id = _text(
                ref.get("semantic_ref"), f"candidate participant[{index}].semantic_ref"
            )
            if family_id not in family_ids:
                family_ids.append(family_id)
    mappings = _array(
        classification.get("source_instance_context_mappings"),
        "classification source-instance mappings",
    )
    for index, raw_mapping in enumerate(mappings):
        mapping = _object(raw_mapping, f"source-instance mapping[{index}]")
        if _array(
            mapping.get("b2_assignment_refs"),
            f"source-instance mapping[{index}].b2_assignment_refs",
        ):
            _fail(
                "UNSUPPORTED_B2_ASSOCIATION",
                "ordinal-zero canary has a B2 assignment edge outside its family-only binding",
            )
    families: list[JsonObject] = []
    for family_id in family_ids:
        try:
            family = resolver.resolve_b2_requirement_family(family_id, bindings)
            boundary = resolver.resolve_b2_boundary(
                family,
                B2BoundaryReferenceV1(
                    family_id=family_id,
                    precise_semantic_definition=_text(
                        family.record.get("precise_semantic_definition"),
                        f"B2 family {family_id} precise semantic definition",
                    ),
                ),
            )
        except ResolutionError as exc:
            raise CanaryPacketError(exc.code, exc.message, exc.status.value) from exc
        families.append(
            {
                "kind": SOURCE_FACT,
                "status": "PASS",
                "family_id": family.family_id,
                "source_binding": _binding_json(family.source_binding),
                "family_record": _plain(family.record),
                "boundary": {
                    "family_id": boundary.boundary_ref.family_id,
                    "precise_semantic_definition": (
                        boundary.boundary_ref.precise_semantic_definition
                    ),
                    "boundary_fields": _plain(family.boundary_fields),
                },
                "semantic_use": "reviewer_reference_inventory_only",
            }
        )
    return {
        "status": "PASS",
        "artifact_bindings": {
            "catalog": _binding_json(bindings.catalog),
            "classifications": _binding_json(bindings.classifications),
            "closure": _binding_json(bindings.closure),
        },
        "requirement_families": families,
        "card_classifications": [],
        "assignment_edges": [],
        "non_applicability_note": (
            "the ordinal-zero participants are requirement families; "
            "no card classification edge is referenced"
        ),
    }


def _b1_inventory(resolver: AuthoritySourceResolver, inputs: LoadedReviewInputs) -> JsonObject:
    bindings = _b1_bindings(inputs)
    try:
        citations_artifact = resolver.resolve_source_binding(bindings.citations)
        closure_artifact = resolver.resolve_source_binding(bindings.closure)
        citations_document = _object(
            json.loads(citations_artifact.raw_bytes.decode("utf-8")),
            "B1.Final citations document",
        )
        closure_document = _object(
            json.loads(closure_artifact.raw_bytes.decode("utf-8")),
            "B1.Final closure document",
        )
        _b1_verify_closure_binding(closure_document, citations_artifact)
        input_universe = _b1_validate_header(citations_document)
        authorities = _array(citations_document.get("authorities"), "B1.Final authorities")
        authority_ids = [
            _text(_object(value, "B1.Final authority").get("authority_id"), "B1.Final authority ID")
            for value in authorities
        ]
        if authority_ids != list(B1_FINAL_AUTHORITY_IDS):
            _fail("B1_INVENTORY_INVALID", "B1.Final authority inventory order differs from V1")
        result: JsonObject = {
            "status": "PASS",
            "citations_binding": _binding_json(bindings.citations),
            "closure_binding": _binding_json(bindings.closure),
            "input_universe": _plain(input_universe),
            "authority_count": len(authorities),
            "authorities": [
                {
                    "kind": SOURCE_FACT,
                    "record": _plain(value),
                    "semantic_use": "reviewer_reference_inventory_only",
                }
                for value in authorities
            ],
            "selected_citations": [],
            "semantic_relevance": "not_established_by_tool",
        }
    except ResolutionError as exc:
        raise CanaryPacketError(exc.code, exc.message, exc.status.value) from exc
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        _fail("B1_INVENTORY_INVALID", f"B1.Final repository artifact is not valid JSON: {exc}")

    try:
        snapshot = resolver._b1_snapshot(bindings)
    except ResolutionError as exc:
        if exc.status is ResolutionStatus.BLOCKED:
            result["official_source_resolution"] = {
                "status": BLOCKED,
                "reason_code": exc.code,
                "detail": (
                    "official REV3 source bytes are unavailable; "
                    "citation nodes remain inventory only"
                ),
            }
            return result
        raise CanaryPacketError(exc.code, exc.message, exc.status.value) from exc

    resolved_citations: list[JsonObject] = []
    for citation_id, (authority_id, citation) in snapshot.citations_by_id.items():
        official = snapshot.official_artifacts_by_id[authority_id]
        if official is None:
            _fail("B1_INVENTORY_INVALID", f"citation {citation_id!r} lacks an official artifact")
        try:
            locator = resolver._resolve_b1_final_official_locator(
                authority_id, citation_id, citation, official
            )
        except ResolutionError as exc:
            raise CanaryPacketError(exc.code, exc.message, exc.status.value) from exc
        resolved_citations.append(
            {
                "kind": SOURCE_FACT,
                "authority_id": authority_id,
                "citation_id": citation_id,
                "citation": _plain(citation),
                "official_artifact": {
                    "path": official.path,
                    "raw_sha256": official.raw_sha256,
                },
                "locator": _plain(locator.locator),
                "resolved_bytes_base64": base64.b64encode(locator.resolved_bytes).decode("ascii"),
                "semantic_use": "reviewer_reference_inventory_only",
            }
        )
    result["official_source_resolution"] = {"status": "PASS"}
    result["citations"] = resolved_citations
    return result


def _worksheet(inputs: LoadedReviewInputs, manifest: Mapping[str, object]) -> JsonObject:
    domains = list(inputs.review_domains)
    temporal = [
        _text(key, "temporal review slot")
        for key in _object(inputs.model.get("temporal_value_vocabulary"), "temporal vocabulary")
    ]
    return {
        "record_type": "non_authoritative_canary_review_worksheet",
        "format": PACKET_FORMAT,
        "authority_status": "non_authoritative",
        "canary": _plain(manifest["canary"]),
        "review_state": AWAITING_HUMAN_REVIEW,
        "reviewer_decisions": {
            "relation_review": {
                "kind": REVIEWER_DECISION,
                "status": AWAITING_HUMAN_REVIEW,
                "relation_conclusion": AWAITING_HUMAN_REVIEW,
                "proof_kind": AWAITING_HUMAN_REVIEW,
                "causal_mechanism": AWAITING_HUMAN_REVIEW,
                "preconditions": AWAITING_HUMAN_REVIEW,
                "evidence_selection": AWAITING_HUMAN_REVIEW,
                "rationale": AWAITING_HUMAN_REVIEW,
            },
            "domain_reviews": [
                {
                    "kind": REVIEWER_DECISION,
                    "review_domain": domain,
                    "status": AWAITING_HUMAN_REVIEW,
                    "applicability": AWAITING_HUMAN_REVIEW,
                    "evidence_selection": AWAITING_HUMAN_REVIEW,
                    "rationale": AWAITING_HUMAN_REVIEW,
                }
                for domain in domains
            ],
            "conditional_context_review": {
                "kind": REVIEWER_DECISION,
                "status": AWAITING_HUMAN_REVIEW,
                "activation": "only_if_the_human_semantic_result_requires_context",
                "dimensions": [
                    {
                        "dimension": dimension,
                        "value": AWAITING_HUMAN_REVIEW,
                        "evidence_selection": AWAITING_HUMAN_REVIEW,
                    }
                    for dimension in SOURCE_CONTEXT_KEYS
                ],
                "temporal_semantics": [
                    {
                        "slot": slot,
                        "value": AWAITING_HUMAN_REVIEW,
                        "evidence_selection": AWAITING_HUMAN_REVIEW,
                    }
                    for slot in temporal
                ],
            },
            "conditional_scope_review": {
                "kind": REVIEWER_DECISION,
                "status": AWAITING_HUMAN_REVIEW,
                "model_boundary": AWAITING_HUMAN_REVIEW,
                "reason": AWAITING_HUMAN_REVIEW,
                "evidence_selection": AWAITING_HUMAN_REVIEW,
            },
        },
    }


def _manifest(
    worklist_manifest: Mapping[str, object],
    worklist_sha256: str,
    item: Mapping[str, object],
    inputs: LoadedReviewInputs,
    status: str,
    source_resolution: Mapping[str, object],
) -> JsonObject:
    canary = {
        "ordinal": CANARY_ORDINAL,
        "candidate_id": item["candidate_id"],
        "candidate_identity": _plain(item["candidate_identity"]),
        "source_instance_id": item["source_instance_id"],
        "scope": item["scope"],
        "relation": item["relation"],
    }
    shard_bindings = _array(
        worklist_manifest.get("classification_shards"), "worklist shard bindings"
    )
    return {
        "record_type": "canary_review_packet_manifest",
        "format": PACKET_FORMAT,
        "packet_status": status,
        "authority_status": "non_authoritative",
        "source_commit": inputs.source_commit,
        "worklist": {
            "format": worklist_manifest["format"],
            "path": "dist/m2-5-c-authority-review/review_worklist.v1.jsonl",
            "raw_sha256": worklist_sha256,
            "ordinal": CANARY_ORDINAL,
        },
        "canary": canary,
        "source_bindings": {
            "declared_model": _plain(worklist_manifest["declared_model"]),
            "candidate_universe": _plain(worklist_manifest["candidate_universe"]),
            "classification_root": _plain(worklist_manifest["classification_root"]),
            "classification_shard": _plain(shard_bindings[0]),
            "classification_shards": _plain(shard_bindings),
            "semantic_classes": _plain(worklist_manifest["semantic_classes"]),
            "current_c_closure": _plain(worklist_manifest["current_c_closure"]),
            "reviewer_roster": _plain(worklist_manifest["reviewer_roster"]),
            "b2_artifacts": _plain(
                _object(
                    inputs.candidate_universe.get("input_bindings"), "candidate input bindings"
                )["b2_artifacts"]
            ),
            "b1_final_artifacts": _plain(
                _object(
                    inputs.candidate_universe.get("input_bindings"), "candidate input bindings"
                )["b1_final_artifacts"]
            ),
        },
        "review_checklist_id": CHECKLIST_ID,
        "source_resolution": _plain(source_resolution),
    }


def _source_inventory(
    candidate: Mapping[str, object],
    instance: Mapping[str, object],
    classification: Mapping[str, object],
    rev3_fact: Mapping[str, object],
    b2_fact: Mapping[str, object],
    b1_fact: Mapping[str, object],
    status: str,
) -> JsonObject:
    return {
        "record_type": "canary_source_inventory",
        "format": PACKET_FORMAT,
        "inventory_status": status,
        "authority_status": "non_authoritative",
        "facts": {
            "candidate": {
                "kind": SOURCE_FACT,
                "verification_status": "C_LEDGER_VERIFIED",
                "record": _plain(candidate),
            },
            "source_instance": {
                "kind": SOURCE_FACT,
                "verification_status": "C_LEDGER_VERIFIED",
                "record": _plain(instance),
                "participant_bindings": _plain(instance["participant_bindings"]),
                "source_context": _plain(instance["source_context"]),
            },
            "rev3": _plain(rev3_fact),
            "current_unresolved_classification": _classification_fact(classification),
            "b2": _plain(b2_fact),
            "b1_final": _plain(b1_fact),
        },
        "reviewer_decision_boundary": "inventory only; no source fact is a semantic conclusion",
    }


def _markdown(manifest: Mapping[str, object], inventory: Mapping[str, object]) -> bytes:
    canary = _object(manifest["canary"], "manifest canary")
    resolution = _object(manifest["source_resolution"], "manifest source resolution")
    facts = _object(inventory.get("facts"), "source inventory facts")
    classification = _object(
        facts.get("current_unresolved_classification"),
        "current unresolved classification",
    )
    lines = [
        "# M2.5.C Single-Candidate Human Review Canary Packet V1",
        "",
        "This packet is non-authoritative review input. It is not a semantic proof,",
        "acceptance event, authority record, or C classification.",
        "",
        f"- packet status: `{manifest['packet_status']}`",
        f"- source commit: `{manifest['source_commit']}`",
        f"- worklist ordinal: `{canary['ordinal']}`",
        f"- candidate ID: `{canary['candidate_id']}`",
        f"- source instance ID: `{canary['source_instance_id']}`",
        f"- scope source fact: `{canary['scope']}`",
        f"- relation source fact: `{canary['relation']}`",
        f"- review checklist: `{manifest['review_checklist_id']}`",
        "",
        "## Source facts",
        "",
        "The machine-readable inventory records exact repository/source facts and",
        "keeps reviewer references separate from human decisions.",
        "",
        f"- REV3 resolution: `{resolution.get('rev3')}`",
        f"- B2 inventory: `{resolution.get('b2')}`",
        f"- B1.Final inventory: `{resolution.get('b1_final')}`",
        f"- current C review state: `{classification.get('review_state')}`",
        "",
        "## Human review boundary",
        "",
        "All relation, domain, conditional context, and conditional scope decision",
        "slots begin as `AWAITING_HUMAN_REVIEW`. No semantic conclusion or human",
        "acceptance is emitted by this packet builder.",
        "",
    ]
    return "\n".join(lines).encode("utf-8")


def _machine_packet_sha256(packet_dir: Path) -> str:
    digest = hashlib.sha256()
    for name in PACKET_FILES:
        raw = (packet_dir / name).read_bytes()
        digest.update(name.encode("utf-8"))
        digest.update(b"\0")
        digest.update(len(raw).to_bytes(8, "big"))
        digest.update(raw)
    return digest.hexdigest()


def _reject_forbidden(value: object, label: str) -> None:
    if isinstance(value, Mapping):
        for key, child in value.items():
            if key in _FORBIDDEN_KEYS:
                _fail("PACKET_QUARANTINE_INVALID", f"{label} contains forbidden field {key!r}")
            if isinstance(child, str) and child in _FORBIDDEN_VALUES:
                _fail("PACKET_QUARANTINE_INVALID", f"{label}.{key} contains a semantic conclusion")
            _reject_forbidden(child, f"{label}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            _reject_forbidden(child, f"{label}[{index}]")


def _canary_records(
    inputs: LoadedReviewInputs,
) -> tuple[Mapping[str, object], Mapping[str, object], Mapping[str, object]]:
    classification = inputs.classification_records[CANARY_ORDINAL]
    candidate_id = _text(classification.get("candidate_id"), "canary candidate ID")
    candidate = next(
        record for record in inputs.candidate_records if record.get("candidate_id") == candidate_id
    )
    instance = next(
        record
        for record in inputs.source_instance_records
        if record.get("candidate_id") == candidate_id
    )
    return candidate, instance, classification


def _expected_packet_canary(inputs: LoadedReviewInputs) -> JsonObject:
    candidate, instance, _ = _canary_records(inputs)
    return {
        "ordinal": CANARY_ORDINAL,
        "candidate_id": candidate["candidate_id"],
        "candidate_identity": _plain(candidate["candidate_identity"]),
        "source_instance_id": instance["source_instance_id"],
        "scope": candidate["scope"],
        "relation": candidate["relation"],
    }


def _expected_packet_bindings(inputs: LoadedReviewInputs) -> JsonObject:
    raw_inputs = _object(
        inputs.candidate_universe.get("input_bindings"), "candidate input bindings"
    )
    return {
        "declared_model": _plain(inputs.model_binding),
        "candidate_universe": _plain(inputs.candidate_universe_binding),
        "classification_root": _plain(inputs.classification_root_binding),
        "classification_shard": _plain(inputs.classification_shard_bindings[0]),
        "classification_shards": _plain(inputs.classification_shard_bindings),
        "semantic_classes": _plain(inputs.semantic_classes_binding),
        "current_c_closure": _plain(inputs.current_c_closure_binding),
        "reviewer_roster": _plain(inputs.reviewer_roster_ref),
        "b2_artifacts": _plain(raw_inputs["b2_artifacts"]),
        "b1_final_artifacts": _plain(raw_inputs["b1_final_artifacts"]),
    }


def _b1_status(inventory: Mapping[str, object]) -> str:
    official = _object(
        inventory.get("official_source_resolution", {"status": "PASS"}),
        "B1 official source resolution",
    )
    return _text(official.get("status"), "B1 resolution status")


def _revalidate_sources(
    inputs: LoadedReviewInputs,
    resolver: AuthoritySourceResolver,
) -> tuple[JsonObject, JsonObject, JsonObject, str, JsonObject]:
    candidate, instance, classification = _canary_records(inputs)
    resolved_source, rev3_fact = _resolve_rev3(resolver, inputs, candidate, instance)
    b2_fact = _resolve_b2_inventory(resolver, inputs, candidate, classification)
    b1_fact = _b1_inventory(resolver, inputs)
    rev3_status = _text(rev3_fact.get("status"), "REV3 resolution status")
    b1_status = _b1_status(b1_fact)
    status = (
        READY_FOR_HUMAN_REVIEW
        if resolved_source is not None and rev3_status == "PASS" and b1_status == "PASS"
        else BLOCKED
    )
    source_resolution = {
        "rev3": rev3_status,
        "b2": b2_fact["status"],
        "b1_final": b1_status,
    }
    return rev3_fact, b2_fact, b1_fact, status, source_resolution


def _qualify_machine_files(
    packet_dir: Path,
    repo_root: Path,
    inputs: LoadedReviewInputs | None,
    resolver: AuthoritySourceResolver | None,
) -> str:
    try:
        entries = {entry.name for entry in packet_dir.iterdir()}
    except OSError as exc:
        _fail("PACKET_INVALID", f"cannot enumerate packet files: {exc}")
    if entries != set(ALL_PACKET_FILES):
        _fail("PACKET_INVALID", "packet directory contains an unexpected file set")
    try:
        values = {
            name: json.loads((packet_dir / name).read_text(encoding="utf-8"))
            for name in PACKET_FILES
        }
    except (OSError, json.JSONDecodeError) as exc:
        _fail("PACKET_INVALID", f"cannot read packet machine files: {exc}")
    for name, value in values.items():
        _reject_forbidden(value, name)
    manifest = _object(values["manifest.v1.json"], "packet manifest")
    if (
        manifest.get("format") != PACKET_FORMAT
        or manifest.get("authority_status") != "non_authoritative"
    ):
        _fail("PACKET_INVALID", "packet manifest is not the non-authoritative V1 format")
    status = _text(manifest.get("packet_status"), "packet status")
    if status not in {READY_FOR_HUMAN_REVIEW, BLOCKED}:
        _fail("PACKET_INVALID", "packet status is not a qualification status")
    canary = _object(manifest.get("canary"), "packet canary")
    if canary.get("ordinal") != CANARY_ORDINAL:
        _fail("CANARY_SELECTION_INVALID", "packet canary is not ordinal zero")
    inventory = _object(values["source_inventory.v1.json"], "source inventory")
    if (
        inventory.get("format") != PACKET_FORMAT
        or inventory.get("authority_status") != "non_authoritative"
        or inventory.get("inventory_status") != status
    ):
        _fail("PACKET_INVALID", "source inventory is authoritative")
    worksheet = _object(values["review_worksheet.v1.json"], "review worksheet")
    if (
        worksheet.get("format") != PACKET_FORMAT
        or worksheet.get("authority_status") != "non_authoritative"
        or worksheet.get("review_state") != AWAITING_HUMAN_REVIEW
        or worksheet.get("canary") != canary
    ):
        _fail("WORKSHEET_INVALID", "worksheet header is not the non-authoritative V1 shape")
    decisions = _object(worksheet.get("reviewer_decisions"), "reviewer decisions")
    if set(decisions) != {
        "relation_review",
        "domain_reviews",
        "conditional_context_review",
        "conditional_scope_review",
    }:
        _fail("WORKSHEET_INVALID", "review decision sections are not closed")
    relation = _object(decisions.get("relation_review"), "relation review")
    if set(relation) != {
        "kind",
        "status",
        "relation_conclusion",
        "proof_kind",
        "causal_mechanism",
        "preconditions",
        "evidence_selection",
        "rationale",
    }:
        _fail("WORKSHEET_INVALID", "relation review fields are not closed")
    for key, value in relation.items():
        if key != "kind" and value != AWAITING_HUMAN_REVIEW:
            _fail("WORKSHEET_INVALID", f"relation slot {key!r} is not awaiting human review")
    domains = _array(decisions.get("domain_reviews"), "domain reviews")
    if len(domains) != 11:
        _fail("WORKSHEET_INVALID", "worksheet does not contain exactly eleven domain reviews")
    seen_domains: set[str] = set()
    for index, raw_domain in enumerate(domains):
        domain = _object(raw_domain, f"domain review[{index}]")
        name = _text(domain.get("review_domain"), f"domain review[{index}].review_domain")
        if name in seen_domains:
            _fail("WORKSHEET_INVALID", "worksheet repeats a review domain")
        seen_domains.add(name)
        if set(domain) != {
            "kind",
            "review_domain",
            "status",
            "applicability",
            "evidence_selection",
            "rationale",
        }:
            _fail("WORKSHEET_INVALID", "domain review fields are not closed")
        for key in ("status", "applicability", "evidence_selection", "rationale"):
            if domain.get(key) != AWAITING_HUMAN_REVIEW:
                _fail("WORKSHEET_INVALID", f"domain slot {key!r} is not awaiting human review")
    context = _object(decisions.get("conditional_context_review"), "context review")
    if context.get("status") != AWAITING_HUMAN_REVIEW:
        _fail("WORKSHEET_INVALID", "context review is not awaiting human review")
    dimensions = _array(context.get("dimensions"), "context dimensions")
    if len(dimensions) != len(SOURCE_CONTEXT_KEYS):
        _fail("WORKSHEET_INVALID", "context review does not contain all ten dimensions")
    for expected, raw_dimension in zip(SOURCE_CONTEXT_KEYS, dimensions, strict=True):
        dimension = _object(raw_dimension, f"context dimension {expected}")
        if set(dimension) != {"dimension", "value", "evidence_selection"}:
            _fail("WORKSHEET_INVALID", "context dimension fields are not closed")
        if (
            dimension.get("dimension") != expected
            or dimension.get("value") != AWAITING_HUMAN_REVIEW
            or dimension.get("evidence_selection") != AWAITING_HUMAN_REVIEW
        ):
            _fail("WORKSHEET_INVALID", f"context dimension {expected!r} is not unresolved")
    temporal = _array(context.get("temporal_semantics"), "temporal semantics")
    if len(temporal) != 4:
        _fail("WORKSHEET_INVALID", "context review does not contain four temporal slots")
    seen_temporal: set[str] = set()
    for index, raw_slot in enumerate(temporal):
        slot = _object(raw_slot, f"temporal slot[{index}]")
        if set(slot) != {"slot", "value", "evidence_selection"}:
            _fail("WORKSHEET_INVALID", "temporal slot fields are not closed")
        slot_name = _text(slot.get("slot"), f"temporal slot[{index}].slot")
        if slot_name in seen_temporal:
            _fail("WORKSHEET_INVALID", "worksheet repeats a temporal slot")
        seen_temporal.add(slot_name)
        if (
            slot.get("value") != AWAITING_HUMAN_REVIEW
            or slot.get("evidence_selection") != AWAITING_HUMAN_REVIEW
        ):
            _fail("WORKSHEET_INVALID", f"temporal slot {slot_name!r} is not unresolved")
    scope = _object(decisions.get("conditional_scope_review"), "scope review")
    if set(scope) != {
        "kind",
        "status",
        "model_boundary",
        "reason",
        "evidence_selection",
    }:
        _fail("WORKSHEET_INVALID", "scope review fields are not closed")
    for key in ("status", "model_boundary", "reason", "evidence_selection"):
        if scope.get(key) != AWAITING_HUMAN_REVIEW:
            _fail("WORKSHEET_INVALID", f"scope slot {key!r} is not awaiting human review")
    loaded = inputs or load_review_inputs(repo_root)
    if manifest.get("source_commit") != loaded.source_commit:
        _fail("PACKET_SOURCE_BINDING_MISMATCH", "packet source commit differs from the repository")
    if manifest.get("review_checklist_id") != CHECKLIST_ID:
        _fail("PACKET_INVALID", "packet checklist identifier is not the accepted V1 checklist")
    expected_canary = _expected_packet_canary(loaded)
    if canary != expected_canary:
        _fail(
            "PACKET_SOURCE_BINDING_MISMATCH",
            "packet canary differs from accepted source facts",
        )
    if worksheet != _worksheet(loaded, manifest):
        _fail(
            "WORKSHEET_SOURCE_BINDING_MISMATCH",
            "worksheet differs from the canonical accepted-model worksheet",
        )
    if manifest.get("source_bindings") != _expected_packet_bindings(loaded):
        _fail(
            "PACKET_SOURCE_BINDING_MISMATCH",
            "packet source bindings differ from accepted inputs",
        )
    worklist = _object(manifest.get("worklist"), "packet worklist")
    expected_worklist_sha = hashlib.sha256(_expected_worklist_bytes(loaded)).hexdigest()
    if worklist != {
        "format": WORKLIST_FORMAT,
        "path": "dist/m2-5-c-authority-review/review_worklist.v1.jsonl",
        "raw_sha256": expected_worklist_sha,
        "ordinal": CANARY_ORDINAL,
    }:
        _fail(
            "PACKET_SOURCE_BINDING_MISMATCH",
            "packet worklist is not the accepted source snapshot",
        )
    candidate, instance, classification = _canary_records(loaded)
    facts = _object(inventory.get("facts"), "source inventory facts")
    candidate_fact = _object(facts.get("candidate"), "candidate source fact")
    instance_fact = _object(facts.get("source_instance"), "source-instance source fact")
    if (
        candidate_fact.get("record") != _plain(candidate)
        or instance_fact.get("record") != _plain(instance)
        or facts.get("current_unresolved_classification") != _classification_fact(classification)
    ):
        _fail("PACKET_SOURCE_FACT_MISMATCH", "packet C source facts differ from accepted inputs")
    source_resolver = resolver or AuthoritySourceResolver(repo_root)
    actual_rev3, actual_b2, actual_b1, actual_status, actual_resolution = _revalidate_sources(
        loaded, source_resolver
    )
    expected_inventory = _source_inventory(
        candidate,
        instance,
        classification,
        actual_rev3,
        actual_b2,
        actual_b1,
        actual_status,
    )
    if inventory != expected_inventory:
        _fail(
            "PACKET_SOURCE_FACT_MISMATCH",
            "source inventory differs from the canonical verified inventory",
        )
    if manifest.get("source_resolution") != actual_resolution:
        _fail(
            "PACKET_SOURCE_RESOLUTION_MISMATCH",
            "packet source-resolution statuses differ from resolver output",
        )
    if actual_status == BLOCKED:
        return BLOCKED
    if status != actual_status:
        _fail("PACKET_STATUS_MISMATCH", "packet status differs from verified source resolution")
    return actual_status


def qualify_packet(
    packet_dir: Path,
    repo_root: Path = ROOT,
    *,
    inputs: LoadedReviewInputs | None = None,
    resolver: AuthoritySourceResolver | None = None,
) -> str:
    """Return structural qualification without assigning semantic authority."""

    return _qualify_machine_files(packet_dir, repo_root, inputs, resolver)


def build_canary_packet(
    repo_root: Path = ROOT,
    output_dir: Path | None = None,
    *,
    inputs: LoadedReviewInputs | None = None,
    resolver: AuthoritySourceResolver | None = None,
) -> CanaryPacketResult:
    """Build the ordinal-zero canary packet under a quarantined output directory."""

    loaded = inputs or load_review_inputs(repo_root)
    worklist_path = build_worklist(repo_root, inputs=loaded)
    worklist_manifest, worklist_item, worklist_sha256 = _read_worklist_canary(worklist_path, loaded)
    candidate_id = _text(worklist_item.get("candidate_id"), "canary candidate ID")
    candidate = next(
        record for record in loaded.candidate_records if record.get("candidate_id") == candidate_id
    )
    instance = next(
        record
        for record in loaded.source_instance_records
        if record.get("candidate_id") == candidate_id
    )
    classification = loaded.classification_records[CANARY_ORDINAL]
    source_resolver = resolver or AuthoritySourceResolver(repo_root)
    rev3_fact, b2_fact, b1_fact, status, source_resolution = _revalidate_sources(
        loaded, source_resolver
    )
    manifest = _manifest(
        worklist_manifest, worklist_sha256, worklist_item, loaded, status, source_resolution
    )
    inventory = _source_inventory(
        candidate, instance, classification, rev3_fact, b2_fact, b1_fact, status
    )
    worksheet = _worksheet(loaded, manifest)
    target_dir = (
        output_dir or repo_root / "dist" / "m2-5-c-authority-review" / PACKET_DIRECTORY_NAME
    )
    target_dir.mkdir(parents=True, exist_ok=True)
    for name, value in (
        ("manifest.v1.json", manifest),
        ("source_inventory.v1.json", inventory),
        ("review_worksheet.v1.json", worksheet),
    ):
        (target_dir / name).write_bytes(_json_bytes(value))
    (target_dir / "REVIEW_PACKET.md").write_bytes(_markdown(manifest, inventory))
    qualified = qualify_packet(target_dir, repo_root, inputs=loaded, resolver=source_resolver)
    if qualified != status:
        _fail("PACKET_STATUS_MISMATCH", "packet status differs from structural qualification")
    return CanaryPacketResult(
        status=qualified,
        packet_dir=target_dir,
        worklist_path=worklist_path,
        worklist_sha256=worklist_sha256,
        packet_sha256=_machine_packet_sha256(target_dir),
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    try:
        result = build_canary_packet(ROOT)
    except CanaryPacketError as exc:
        print(f"{exc.status}: {exc.code}: {exc.message}", file=sys.stderr)
        return 2 if exc.status == BLOCKED else 1
    print(
        f"{result.status}: generated {result.packet_dir.relative_to(ROOT)} "
        f"worklist_sha256={result.worklist_sha256} packet_sha256={result.packet_sha256}"
    )
    return 2 if result.status == BLOCKED else 0


if __name__ == "__main__":
    raise SystemExit(main())
