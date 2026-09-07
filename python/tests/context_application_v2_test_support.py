from __future__ import annotations

import hashlib
import json
import sys
from collections.abc import Mapping
from dataclasses import replace
from pathlib import Path
from typing import Any, cast

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT / "scripts") not in sys.path:
    sys.path.insert(0, str(ROOT / "scripts"))

from authority_source_resolver import AuthoritySourceResolver
from context_application_v2_resolver import (
    ContextApplicationV2Resolver,
    context_source_binding_from_wire,
)
from mtgml.authority import (
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKindV3,
    AcceptanceSubjectPayloadV3,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationV2Record,
    ContextApplicationV2InputV1,
    ContextApplicationV2SupersessionInputV2,
    ContextApplicationV2SupersessionRecord,
    DigestReferenceV1,
    ReviewAcceptanceEventInputV3,
    ReviewAcceptanceEventLeafV3,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewEventRefV3,
    ReviewMode,
)


DEFAULT_REVIEWER_ROLES: tuple[str, ...] = (
    "architecture_maintainer",
    "conformance_maintainer",
    "information_safety_reviewer",
    "rules_authority_maintainer",
)


def _write_event_for_subject(
    case: Mapping[str, object],
    subject: ContextApplicationV2Record | ContextApplicationV2SupersessionRecord,
    *,
    subject_kind: AcceptanceSubjectKindV3,
    review_mode: ReviewMode,
    reviewer_roles: tuple[str, ...],
) -> tuple[ReviewEventRefV3, dict[str, object]]:
    fixture = cast(Any, case["fixture"])
    source_resolver = cast(AuthoritySourceResolver, case["source_resolver"])
    base_binding = cast(object, case["base_binding"])
    reviewer_roles = tuple(sorted(reviewer_roles))

    roster_raw = json.dumps(
        {
            "schema": "manafold.m2.5.c.reviewer-roster.v1",
            "reviewers": [
                {
                    "reviewer_id": "alice",
                    "roles": list(reviewer_roles),
                }
            ],
        },
        separators=(",", ":"),
    ).encode("utf-8")
    roster_digest = hashlib.sha256(roster_raw).digest()
    roster_path = (
        "sources/m2_5/authorities/reviewer_rosters/v1/" + roster_digest.hex() + ".json"
    )
    fixture.write_repo(roster_path, roster_raw)
    roster_ref = ReviewerRosterRefV1(
        roster_path,
        "manafold.m2.5.c.reviewer-roster.v1",
        roster_digest,
    )

    evidence_path = "docs/review/context-application-v2-slice5.md"
    evidence_raw = b"synthetic Slice-5 review evidence" + bytes([10])
    fixture.write_repo(evidence_path, evidence_raw)
    evidence_ref = AcceptanceEvidenceRefV1(
        evidence_path,
        hashlib.sha256(evidence_raw).digest(),
        ("whole_artifact", None),
    )

    resolver = ContextApplicationV2Resolver(
        source_resolver,
        base_authority_binding=base_binding,
    )
    closure = resolver.expected_acceptance_source_closure_v3(subject, roster_ref)
    acceptance_subject = AcceptanceSubjectPayloadV3(
        subject_kind=subject_kind,
        subject_payload=subject.acceptance_free_subject_payload(),
    )
    event_input = ReviewAcceptanceEventInputV3(
        subject_kind=subject_kind,
        subject_payload_digest_reference=DigestReferenceV1.from_identity(
            acceptance_subject.identity()
        ),
        reviewer_roster_ref=roster_ref,
        reviewer_role_bindings=(
            ReviewerRoleBindingV1(
                "alice",
                reviewer_roles,
            ),
        ),
        review_mode=review_mode,
        source_binding_digests=closure,
        review_evidence_refs=(evidence_ref,),
    )
    event_wire = ReviewAcceptanceEventLeafV3.from_input(event_input).to_wire()
    event_raw = json.dumps(event_wire, separators=(",", ":")).encode("utf-8") + bytes([10])
    event_id = cast(str, event_wire["event_id"])
    event_path = (
        "sources/m2_5/authorities/review_acceptance_events/v3/"
        + event_id.removeprefix("ae.v3/")
        + ".json"
    )
    fixture.write_repo(event_path, event_raw)
    return (
        ReviewEventRefV3(event_path, hashlib.sha256(event_raw).digest(), event_id),
        event_wire,
    )


def _digest_reference_from_wire(value: object) -> DigestReferenceV1:
    record = cast(dict[str, object], value)
    return DigestReferenceV1(
        cast(str, record["envelope_id"]),
        cast(str, record["algorithm_id"]),
        cast(str, record["semantic_domain"]),
        cast(str, record["payload_codec_id"]),
        cast(str, record["input_schema_id"]),
        bytes.fromhex(cast(str, record["digest_hex"])),
    )


def _acceptance_locator(value: dict[str, object]) -> tuple[str, str | None]:
    kind = cast(str, value["kind"])
    if kind == "whole_artifact":
        return (kind, None)
    return (kind, cast(str, value["value"]))


def _event_input_from_wire(wire: dict[str, object]) -> ReviewAcceptanceEventInputV3:
    roster_wire = cast(dict[str, object], wire["reviewer_roster_ref"])
    bindings = tuple(
        ReviewerRoleBindingV1(
            cast(str, cast(dict[str, object], item)["reviewer_id"]),
            tuple(cast(list[str], cast(dict[str, object], item)["roles"])),
        )
        for item in cast(list[object], wire["reviewer_role_bindings"])
    )
    sources = tuple(
        context_source_binding_from_wire(item)
        for item in cast(list[object], wire["source_binding_digests"])
    )
    evidence = tuple(
        AcceptanceEvidenceRefV1(
            cast(str, cast(dict[str, object], item)["path"]),
            bytes.fromhex(cast(str, cast(dict[str, object], item)["raw_sha256"])),
            _acceptance_locator(
                cast(dict[str, object], cast(dict[str, object], item)["locator"])
            ),
        )
        for item in cast(list[object], wire["review_evidence_refs"])
    )
    return ReviewAcceptanceEventInputV3(
        subject_kind=AcceptanceSubjectKindV3(cast(str, wire["subject_kind"])),
        subject_payload_digest_reference=_digest_reference_from_wire(
            wire["subject_payload_digest"]
        ),
        reviewer_roster_ref=ReviewerRosterRefV1(
            cast(str, roster_wire["path"]),
            cast(str, roster_wire["schema"]),
            bytes.fromhex(cast(str, roster_wire["raw_sha256"])),
        ),
        reviewer_role_bindings=bindings,
        review_mode=ReviewMode(cast(str, wire["review_mode"])),
        source_binding_digests=sources,
        review_evidence_refs=evidence,
    )


def rebind_application_event(
    case: Mapping[str, object],
    record: ContextApplicationV2Record,
    wire: dict[str, object],
) -> ContextApplicationV2Record:
    fixture = cast(Any, case["fixture"])
    event_input = _event_input_from_wire(wire)
    event_wire = ReviewAcceptanceEventLeafV3.from_input(event_input).to_wire()
    raw = json.dumps(event_wire, separators=(",", ":")).encode("utf-8") + bytes([10])
    event_id = cast(str, event_wire["event_id"])
    event_path = (
        "sources/m2_5/authorities/review_acceptance_events/v3/"
        + event_id.removeprefix("ae.v3/")
        + ".json"
    )
    fixture.write_repo(event_path, raw)
    event_ref = ReviewEventRefV3(event_path, hashlib.sha256(raw).digest(), event_id)
    return ContextApplicationV2Record.from_parts(
        application_id=record.application_id,
        theorem_record_id=record.theorem_record_id,
        members=record.members,
        review_event_ref_v3=event_ref,
    )


def rebind_supersession_event(
    case: Mapping[str, object],
    record: ContextApplicationV2SupersessionRecord,
    wire: dict[str, object],
) -> ContextApplicationV2SupersessionRecord:
    fixture = cast(Any, case["fixture"])
    event_input = _event_input_from_wire(wire)
    event_wire = ReviewAcceptanceEventLeafV3.from_input(event_input).to_wire()
    raw = json.dumps(event_wire, separators=(",", ":")).encode("utf-8") + bytes([10])
    event_id = cast(str, event_wire["event_id"])
    event_path = (
        "sources/m2_5/authorities/review_acceptance_events/v3/"
        + event_id.removeprefix("ae.v3/")
        + ".json"
    )
    fixture.write_repo(event_path, raw)
    event_ref = ReviewEventRefV3(event_path, hashlib.sha256(raw).digest(), event_id)
    return ContextApplicationV2SupersessionRecord.from_parts(
        supersession_id=record.supersession_id,
        superseded_record_id=record.superseded_record_id,
        replacement_record_id=record.replacement_record_id,
        reason_code=record.reason_code,
        source_evidence_refs=record.source_evidence_refs,
        review_event_ref_v3=event_ref,
    )


def build_application_with_v3_event(
    test_case: object,
    case: Mapping[str, object],
    *,
    review_mode: ReviewMode = ReviewMode.MULTI_REVIEWER,
    reviewer_roles: tuple[str, ...] = DEFAULT_REVIEWER_ROLES,
) -> tuple[AuthoritySourceResolver, ContextApplicationV2Record, dict[str, object]]:
    del test_case
    initial_record = cast(ContextApplicationV2Record, case["record"])
    zero_ref = ReviewEventRefV3(
        "sources/m2_5/authorities/review_acceptance_events/v3/" + "00" * 32 + ".json",
        bytes(32),
        "ae.v3/" + "00" * 32,
    )
    provisional = ContextApplicationV2Record.from_parts(
        application_id=initial_record.application_id,
        theorem_record_id=initial_record.theorem_record_id,
        members=initial_record.members,
        review_event_ref_v3=zero_ref,
    )
    event_ref, event_wire = _write_event_for_subject(
        case,
        provisional,
        subject_kind=AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD,
        review_mode=review_mode,
        reviewer_roles=reviewer_roles,
    )
    record = ContextApplicationV2Record.from_parts(
        application_id=initial_record.application_id,
        theorem_record_id=initial_record.theorem_record_id,
        members=initial_record.members,
        review_event_ref_v3=event_ref,
    )
    return cast(AuthoritySourceResolver, case["source_resolver"]), record, event_wire


def build_application_variant_with_v3_event(
    test_case: object,
    case: Mapping[str, object],
    variant_tag: str,
    *,
    review_mode: ReviewMode = ReviewMode.MULTI_REVIEWER,
    reviewer_roles: tuple[str, ...] = DEFAULT_REVIEWER_ROLES,
) -> tuple[AuthoritySourceResolver, ContextApplicationV2Record, dict[str, object]]:
    del test_case
    initial_record = cast(ContextApplicationV2Record, case["record"])
    member = initial_record.members[0]
    bridge = member.context_member_bridge_attestation_v2
    variant_bridge = replace(
        bridge,
        context=tuple(
            replace(slot, rationale=f"{slot.rationale} {variant_tag}")
            for slot in bridge.context
        ),
        temporal=tuple(
            replace(slot, rationale=f"{slot.rationale} {variant_tag}")
            for slot in bridge.temporal
        ),
    )
    variant_member = replace(
        member,
        context_member_bridge_attestation_v2=variant_bridge,
    )
    application_id = ContextApplicationV2InputV1(
        theorem_record_id_bytes=initial_record.theorem_record_id.digest_bytes,
        members=(variant_member,),
    ).identity()
    zero_ref = ReviewEventRefV3(
        "sources/m2_5/authorities/review_acceptance_events/v3/" + "00" * 32 + ".json",
        bytes(32),
        "ae.v3/" + "00" * 32,
    )
    provisional = ContextApplicationV2Record.from_parts(
        application_id=application_id,
        theorem_record_id=initial_record.theorem_record_id,
        members=(variant_member,),
        review_event_ref_v3=zero_ref,
    )
    event_ref, event_wire = _write_event_for_subject(
        case,
        provisional,
        subject_kind=AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD,
        review_mode=review_mode,
        reviewer_roles=reviewer_roles,
    )
    record = ContextApplicationV2Record.from_parts(
        application_id=application_id,
        theorem_record_id=initial_record.theorem_record_id,
        members=(variant_member,),
        review_event_ref_v3=event_ref,
    )
    return cast(AuthoritySourceResolver, case["source_resolver"]), record, event_wire


def build_supersession_with_v3_event(
    test_case: object,
    case: Mapping[str, object],
    semantic_input: ContextApplicationV2SupersessionInputV2,
    *,
    review_mode: ReviewMode = ReviewMode.MULTI_REVIEWER,
    reviewer_roles: tuple[str, ...] = DEFAULT_REVIEWER_ROLES,
) -> tuple[
    AuthoritySourceResolver,
    ContextApplicationV2SupersessionRecord,
    dict[str, object],
]:
    del test_case
    superseded_record_id = AuthorityIdentityV1(
        AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2,
        semantic_input.superseded_record_id_bytes,
    )
    replacement_record_id = (
        None
        if semantic_input.replacement_record_id_bytes is None
        else AuthorityIdentityV1(
            AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2,
            semantic_input.replacement_record_id_bytes,
        )
    )
    zero_ref = ReviewEventRefV3(
        "sources/m2_5/authorities/review_acceptance_events/v3/" + "00" * 32 + ".json",
        bytes(32),
        "ae.v3/" + "00" * 32,
    )
    provisional = ContextApplicationV2SupersessionRecord.from_parts(
        supersession_id=semantic_input.identity(),
        superseded_record_id=superseded_record_id,
        replacement_record_id=replacement_record_id,
        reason_code=semantic_input.reason_code,
        source_evidence_refs=semantic_input.source_evidence_refs,
        review_event_ref_v3=zero_ref,
    )
    event_ref, event_wire = _write_event_for_subject(
        case,
        provisional,
        subject_kind=AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_SUPERSESSION_RECORD,
        review_mode=review_mode,
        reviewer_roles=reviewer_roles,
    )
    record = ContextApplicationV2SupersessionRecord.from_parts(
        supersession_id=provisional.supersession_id,
        superseded_record_id=provisional.superseded_record_id,
        replacement_record_id=provisional.replacement_record_id,
        reason_code=provisional.reason_code,
        source_evidence_refs=provisional.source_evidence_refs,
        review_event_ref_v3=event_ref,
    )
    return cast(AuthoritySourceResolver, case["source_resolver"]), record, event_wire


__all__ = [
    "DEFAULT_REVIEWER_ROLES",
    "build_application_with_v3_event",
    "build_application_variant_with_v3_event",
    "build_supersession_with_v3_event",
    "rebind_application_event",
    "rebind_supersession_event",
]
