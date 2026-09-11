"""Build a fresh, source-bound, non-authoritative five-candidate pilot proposal.

This module only projects current source facts and explicitly proposed review
paths.  It never creates accepted records, V4 events, currentness state, C
artifacts, ranking output, or production authority.
"""

from __future__ import annotations

import json
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Final, NoReturn, cast

from authority_source_resolver import (
    EXPECTED_REV3_ARCHIVE_SHA256,
    AuthoritySourceResolver,
    ResolutionError,
)
from build_m2_5_c_authority_review_worklist import (
    ROOT,
    LoadedReviewInputs,
    load_review_inputs,
)
from mtgml.authority import (
    ParticipantRoleBridgeEntryV1,
    ParticipantRoleBridgeV1,
    SourceBindingDigestV1,
)

PROPOSAL_FORMAT: Final = "manafold.m2.5.c.scaled-authority-pilot-proposal.v1"
PROPOSAL_NAME: Final = "scaled_authority_pilot_proposal.v1.json"
EXPECTED_PILOT_COUNT: Final = 5
SEMANTIC_PROPOSAL_BLOCKER: Final = "SEMANTIC_PROPOSAL_INPUT_MISSING"
SEMANTIC_PROPOSAL_MISSING_FIELDS: Final = (
    "reviewed_relation_proof_v1",
    "causal_or_separation_mechanism",
    "candidate_specific_b1_final_citations",
    "candidate_specific_b2_boundary_evidence",
    "context_and_temporal_semantics",
    "theorem_preconditions",
)


@dataclass(frozen=True)
class PilotSpec:
    ordinal: int
    candidate_id: str
    candidate_identity: str
    source_instance_id: str
    rev3_row_ordinal: int
    human_review_anchor: str
    reviewed_roles: tuple[str, ...] | None = None


PILOT_SPECS: Final[tuple[PilotSpec, ...]] = (
    PilotSpec(
        1,
        "INTRA_DECK|Token Triumph|cap.landfall|cap.token_creation|UNORDERED_BINARY",
        "a953f5368818f62b4f68f7257048cb8bd408563d90383eacd35a7df4e5a1c513",
        "si.v1/SU5UUkFfREVDS3xUb2tlbiBUcml1bXBofGNhcC5sYW5kZmFsbHxjYXAudG9rZW5fY3JlYXRpb258VU5PUkRFUkVEX0JJTkFSWQ/0",
        13199,
        "Sporemound / Token Triumph",
    ),
    PilotSpec(
        2,
        "INTRA_DECK|Grave Danger|cap.graveyard|cap.graveyard_cast_permission|UNORDERED_BINARY",
        "64da95c09ae66c5f7c7a73bc0aa35757869f1e037501c8df063b2137164f22e9",
        "si.v1/SU5UUkFfREVDS3xHcmF2ZSBEYW5nZXJ8Y2FwLmdyYXZleWFyZHxjYXAuZ3JhdmV5YXJkX2Nhc3RfcGVybWlzc2lvbnxVTk9SREVSRURfQklOQVJZ/0",
        10148,
        "Gisa and Geralf / Grave Danger",
    ),
    PilotSpec(
        3,
        "INTRA_DECK|Phantom Premonition|cap.exile|cap.reveal|UNORDERED_BINARY",
        "7ffcc60b503dd9931e048c6a87cbfa5d5f09c976126289f9a26a6a8c65b40dd2",
        "si.v1/SU5UUkFfREVDS3xQaGFudG9tIFByZW1vbml0aW9ufGNhcC5leGlsZXxjYXAucmV2ZWFsfFVOT1JERVJFRF9CSU5BUlk/0",
        11912,
        "Synthetic Destiny / Phantom Premonition",
    ),
    PilotSpec(
        4,
        "CROSS_DECK|P3|cap.mass_destruction|cap.death_trigger|DIRECTIONAL_BINARY",
        "af33dd4f0b65103102828bfec8ebd23196b1685282ee6ef9f0dc4690e3a6420b",
        "si.v1/Q1JPU1NfREVDS3xQM3xjYXAubWFzc19kZXN0cnVjdGlvbnxjYXAuZGVhdGhfdHJpZ2dlcnxESVJFQ1RJT05BTF9CSU5BUlk/0",
        6463,
        "Organic Extinction -> Rampant Rejuvenator",
        ("source", "affected"),
    ),
    PilotSpec(
        5,
        "INTRA_DECK|Upgrades Unleashed|cap.aura|cap.totem_armor_replacement|UNORDERED_BINARY",
        "c4b8417bac1fb70a7c409be2e64b1e0c6bc88dd7fd041ab5f4097ce6a6ba9c96",
        "si.v1/SU5UUkFfREVDS3xVcGdyYWRlcyBVbmxlYXNoZWR8Y2FwLmF1cmF8Y2FwLnRvdGVtX2FybW9yX3JlcGxhY2VtZW50fFVOT1JERVJFRF9CSU5BUlk/0",
        14116,
        "Bear Umbra / Upgrades Unleashed",
    ),
)


class PilotProposalError(ValueError):
    """A fail-closed proposal construction error."""

    def __init__(self, code: str, message: str, status: str = "FAIL") -> None:
        self.code = code
        self.status = status
        super().__init__(f"[{status}:{code}] {message}")


def _fail(code: str, message: str, status: str = "FAIL") -> NoReturn:
    raise PilotProposalError(code, message, status)


def _plain_value(value: object) -> object:
    if isinstance(value, Mapping):
        return {str(key): _plain_value(child) for key, child in value.items()}
    if isinstance(value, list | tuple):
        return [_plain_value(child) for child in value]
    return value


def _plain_mapping(value: Mapping[str, object]) -> dict[str, object]:
    return cast(dict[str, object], _plain_value(value))


def _json_bytes(value: object) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode("utf-8")


def _candidate_binding(inputs: LoadedReviewInputs) -> SourceBindingDigestV1:
    binding = inputs.candidate_universe_binding
    try:
        return SourceBindingDigestV1(
            "candidate_universe",
            cast(str, binding["path"]),
            cast(str, binding["schema"]),
            bytes.fromhex(cast(str, binding["raw_sha256"])),
        )
    except (KeyError, TypeError, ValueError) as exc:
        _fail("CANDIDATE_UNIVERSE_BINDING_INVALID", str(exc))


def _source_instance_map(inputs: LoadedReviewInputs) -> dict[str, Mapping[str, object]]:
    return {cast(str, item["candidate_id"]): item for item in inputs.source_instance_records}


def _candidate_map(inputs: LoadedReviewInputs) -> dict[str, Mapping[str, object]]:
    return {cast(str, item["candidate_id"]): item for item in inputs.candidate_records}


def _classification_map(inputs: LoadedReviewInputs) -> dict[str, Mapping[str, object]]:
    return {cast(str, item["candidate_id"]): item for item in inputs.classification_records}


def _candidate4_bridge(
    source_instance: Mapping[str, object], reviewed_roles: Sequence[str]
) -> list[dict[str, object]]:
    raw_participants = source_instance.get("participant_bindings")
    if not isinstance(raw_participants, list) or len(raw_participants) != 2:
        _fail("CANDIDATE4_ROLE_BRIDGE_INVALID", "Candidate 4 must have exactly two participants")
    if tuple(reviewed_roles) != ("source", "affected"):
        _fail("CANDIDATE4_REVIEWED_ROLE_INVALID", "Candidate 4 reviewed roles are not exact")
    entries: list[ParticipantRoleBridgeEntryV1] = []
    for position, raw in enumerate(raw_participants):
        if not isinstance(raw, Mapping):
            _fail("CANDIDATE4_ROLE_BRIDGE_INVALID", "Candidate 4 participant is not an object")
        ref = raw.get("participant_ref")
        if not isinstance(ref, Mapping) or raw.get("role") != "ordered_participant":
            _fail("CANDIDATE4_HISTORICAL_ROLE_MISMATCH", "Candidate 4 history is not immutable V1")
        try:
            entries.append(
                ParticipantRoleBridgeEntryV1(
                    position=position,
                    participant_kind=cast(str, ref.get("participant_kind")),
                    semantic_ref=cast(str, ref.get("semantic_ref")),
                    historical_source_role=cast(str, raw.get("role")),
                    reviewed_role=reviewed_roles[position],
                )
            )
        except (TypeError, ValueError) as exc:
            _fail("CANDIDATE4_ROLE_BRIDGE_INVALID", str(exc))
    bridge = ParticipantRoleBridgeV1(tuple(entries))
    return [entry.to_wire() for entry in bridge.entries]


def _resolve_candidate(
    spec: PilotSpec,
    candidate: Mapping[str, object],
    source_instance: Mapping[str, object],
    inputs: LoadedReviewInputs,
    resolver: AuthoritySourceResolver,
) -> dict[str, object]:
    identity = candidate.get("candidate_identity")
    if not isinstance(identity, Mapping) or identity.get("digest_hex") != spec.candidate_identity:
        _fail("CANDIDATE_IDENTITY_MISMATCH", spec.candidate_id)
    if source_instance.get("source_instance_id") != spec.source_instance_id:
        _fail("SOURCE_INSTANCE_BINDING_MISMATCH", spec.candidate_id)
    try:
        resolved = resolver.resolve_candidate_source_instance(
            spec.candidate_id,
            cast(Mapping[str, object], identity),
            spec.source_instance_id,
            _candidate_binding(inputs),
        )
    except ResolutionError as exc:
        raise PilotProposalError(exc.code, exc.message, exc.status.value) from exc
    resolved_record = _plain_mapping(cast(Mapping[str, object], resolved.source_instance_record))
    persisted_record = _plain_mapping(source_instance)
    if resolved_record != persisted_record:
        _fail("SOURCE_INSTANCE_RESOLUTION_MISMATCH", spec.candidate_id)
    if resolved.source_binding.get("row_ordinal") != spec.rev3_row_ordinal:
        _fail("REV3_ROW_BINDING_MISMATCH", spec.candidate_id)
    return {
        "candidate": _plain_mapping(candidate),
        "source_instance": persisted_record,
        "resolved_source": {
            "path": resolved.source_artifact.path,
            "raw_sha256": resolved.source_artifact.raw_sha256,
            "source_binding": _plain_mapping(resolved.source_binding),
        },
    }


def _path_for_spec(spec: PilotSpec, source_instance: Mapping[str, object]) -> dict[str, object]:
    if spec.reviewed_roles is None:
        return {
            "status": "blocked",
            "eligibility": "defer_until_exact_relation_proof_v1",
            "blocking_reasons": [SEMANTIC_PROPOSAL_BLOCKER],
            "required_review_families": ["relation_proof_v1", "context_proof_v1"],
        }
    bridge = _candidate4_bridge(source_instance, spec.reviewed_roles)
    return {
        "status": "blocked",
        "eligibility": "role_divergent_requires_v2_v3",
        "relation_application_v2": {
            "status": "deferred",
            "prerequisite": "current_accepted_relation_proof_v1",
            "materialized_id": None,
        },
        "context_application_v3": {
            "status": "deferred",
            "prerequisite": ("current_accepted_context_proof_v1_and_current_rpa_v2_application_id"),
            "materialized_id": None,
        },
        "application_host_binding_v3": {
            "status": "deferred",
            "prerequisite": "exact_context_application_v3_id",
            "deferred_code": "DEFERRED_UNTIL_CONTEXT_APPLICATION_ID",
            "materialized": False,
            "claim_ids": [],
        },
        "host_binding_prerequisite": "DEFERRED_UNTIL_CONTEXT_APPLICATION_ID",
        "relation_application_family": "rpa.v2",
        "relation_application_record_family": "rpar.v2",
        "context_application_family": "cpa.v3",
        "context_application_record_family": "cpar.v3",
        "host_binding_family": "application_host_binding_v3",
        "participant_role_bridge": bridge,
        "deferred_until": "theorem_acceptance",
        "blocking_reasons": [SEMANTIC_PROPOSAL_BLOCKER],
        "acceptance_materialization": "not_authorized",
    }


def build_proposal_document(
    repo_root: Path = ROOT,
    *,
    inputs: LoadedReviewInputs | None = None,
    resolver: AuthoritySourceResolver | None = None,
    pilot_specs: tuple[PilotSpec, ...] = PILOT_SPECS,
) -> dict[str, object]:
    loaded = inputs or load_review_inputs(repo_root)
    if len(pilot_specs) != EXPECTED_PILOT_COUNT or tuple(
        spec.ordinal for spec in pilot_specs
    ) != tuple(range(1, EXPECTED_PILOT_COUNT + 1)):
        _fail("PILOT_SELECTION_INVALID", "pilot must contain exactly five ordered candidates")
    resolver = resolver or AuthoritySourceResolver(repo_root)
    try:
        rev3_archive_sha = resolver._archive().archive_sha256
    except ResolutionError as exc:
        raise PilotProposalError(exc.code, exc.message, exc.status.value) from exc
    if rev3_archive_sha != EXPECTED_REV3_ARCHIVE_SHA256:
        _fail("REV3_ARCHIVE_BINDING_MISMATCH", "configured REV3 archive is not the pinned package")
    candidates = _candidate_map(loaded)
    instances = _source_instance_map(loaded)
    classifications = _classification_map(loaded)
    selected_ids = tuple(spec.candidate_id for spec in pilot_specs)
    if len(set(selected_ids)) != EXPECTED_PILOT_COUNT:
        _fail("PILOT_SELECTION_DUPLICATE", "pilot candidate IDs must be unique")
    result_candidates: list[dict[str, object]] = []
    for spec in pilot_specs:
        candidate = candidates.get(spec.candidate_id)
        if candidate is None:
            _fail("PILOT_SELECTION_MISMATCH", spec.candidate_id)
        instance = instances.get(spec.candidate_id)
        if instance is None:
            _fail("SOURCE_INSTANCE_BINDING_MISMATCH", spec.candidate_id)
        if candidate.get("candidate_id") != spec.candidate_id:
            _fail("PILOT_SELECTION_MISMATCH", spec.candidate_id)
        resolved = _resolve_candidate(spec, candidate, instance, loaded, resolver)
        classification = classifications.get(spec.candidate_id)
        if classification is None:
            _fail("CLASSIFICATION_MISSING", spec.candidate_id)
        result_candidates.append(
            {
                "pilot_ordinal": spec.ordinal,
                "human_review_anchor": spec.human_review_anchor,
                "candidate_id": spec.candidate_id,
                "candidate_identity": spec.candidate_identity,
                "rev3_row_ordinal": spec.rev3_row_ordinal,
                "source_instance_id": spec.source_instance_id,
                "candidate_universe_binding": _plain_mapping(loaded.candidate_universe_binding),
                "candidate": resolved["candidate"],
                "source_instance": resolved["source_instance"],
                "resolved_source": resolved["resolved_source"],
                "current_c_classification": _plain_mapping(classification),
                "proposed_authority_path": _path_for_spec(spec, instance),
                "semantic_status": "blocked",
                "semantic_proposal_missing_fields": list(SEMANTIC_PROPOSAL_MISSING_FIELDS),
                "blocking_reasons": [SEMANTIC_PROPOSAL_BLOCKER],
                "acceptance_status": "not_authorized",
            }
        )
    source_identity = {
        "source_commit": loaded.source_commit,
        "declared_model": _plain_value(loaded.model_binding),
        "candidate_universe": _plain_value(loaded.candidate_universe_binding),
        "current_c_closure": _plain_value(loaded.current_c_closure_binding),
        "classification_root": _plain_value(loaded.classification_root_binding),
        "semantic_classes": _plain_value(loaded.semantic_classes_binding),
        "classification_shards": _plain_value(loaded.classification_shard_bindings),
        "reviewer_roster": _plain_value(loaded.reviewer_roster_ref),
        "rev3_archive_sha256": rev3_archive_sha,
    }
    return {
        "record_type": "non_authoritative_scaled_authority_pilot_proposal",
        "format": PROPOSAL_FORMAT,
        "proposal_state": "blocked",
        "construction_status": "blocked",
        "authority_status": "non_authoritative",
        "acceptance_status": "not_authorized",
        "construction_blockers": [
            {
                "code": SEMANTIC_PROPOSAL_BLOCKER,
                "candidate_ordinals": [spec.ordinal for spec in pilot_specs],
                "missing_fields": list(SEMANTIC_PROPOSAL_MISSING_FIELDS),
                "detail": (
                    "candidate-specific theorem and semantic review inputs are not present; "
                    "no authority may be inferred"
                ),
            }
        ],
        "source_identity": source_identity,
        "pilot": {
            "selection_policy": "fixed-five-candidate-human-anchor-v1",
            "candidate_count": EXPECTED_PILOT_COUNT,
            "candidates": result_candidates,
        },
        "explicit_non_claims": [
            "no human semantic acceptance",
            "no production authority records",
            "no C closure change",
            "no ranking or deck-pair lock",
            "no Domain Audit 55/55",
            "no M3",
        ],
    }


def build_scaled_authority_pilot_proposal(
    repo_root: Path = ROOT,
    output_dir: Path | None = None,
    *,
    inputs: LoadedReviewInputs | None = None,
    resolver: AuthoritySourceResolver | None = None,
) -> Path:
    proposal = build_proposal_document(repo_root, inputs=inputs, resolver=resolver)
    target_dir = output_dir or repo_root / "dist" / "m2-5-c-scaled-authority-pilot-proposal-refresh"
    target_dir.mkdir(parents=True, exist_ok=True)
    target = target_dir / PROPOSAL_NAME
    target.write_bytes(_json_bytes(proposal))
    return target


def main() -> int:
    try:
        path = build_scaled_authority_pilot_proposal()
    except PilotProposalError as exc:
        print(f"{exc.status}: {exc.code}: {exc}")
        return 2 if exc.status == "BLOCKED" else 1
    print(f"PASS: generated non-authoritative proposal {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
