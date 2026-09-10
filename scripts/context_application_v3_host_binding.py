"""ContextApplicationV3 composition with the existing HBC authority."""

from __future__ import annotations

import sys
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from authority_source_resolver import AuthoritySourceResolver, ResolutionError
from authority_v2_validator import (
    AuthorityV2ValidationError,
    AuthorityV2Validator,
    HostBindingAuthorityV2ReadModel,
)
from mtgml.authority import (
    ApplicationHostBindingV3,
    ContextApplicationV3Record,
    ContextAuthoritySourceBindingV3,
)
from mtgml.host_binding import ApplicationMemberKeyV1
from mtgml.persistence import encode_canonical


class ContextApplicationV3HostBindingError(ValueError):
    def __init__(self, code: str, location: str) -> None:
        self.code = code
        self.location = location
        super().__init__(f"{code} at {location}")


@dataclass(frozen=True)
class ContextApplicationV3HostBindingResult:
    application_id: str
    claim_ids: tuple[str, ...]
    required_member_keys: tuple[ApplicationMemberKeyV1, ...]


def required_member_keys(
    record: ContextApplicationV3Record,
    candidate_records: Mapping[str, Mapping[str, object]],
) -> tuple[ApplicationMemberKeyV1, ...]:
    required: list[ApplicationMemberKeyV1] = []
    for member in record.members:
        candidate = candidate_records.get(member.candidate_id)
        if candidate is None:
            raise ContextApplicationV3HostBindingError("HOST_CLAIM_UNKNOWN", "candidate_id")
        required_member = (
            candidate.get("scope") == "cross_deck"
            and candidate.get("relation") == "directional_binary"
        )
        if required_member and member.reviewed_context_binding_v1[3] == "not_applicable":
            raise ContextApplicationV3HostBindingError(
                "HOST_RELATIONSHIP_MISMATCH", "context_binding.host_relationship"
            )
        if required_member:
            required.append(
                ApplicationMemberKeyV1(
                    candidate_id=member.candidate_id,
                    candidate_identity_digest=member.candidate_identity_digest_reference.digest_bytes,
                    source_instance_id=member.source_instance_id,
                )
            )
    return tuple(sorted(required, key=lambda item: encode_canonical(item.to_cbor())))


def validate_application_host_binding_v3(
    record: ContextApplicationV3Record,
    link: ApplicationHostBindingV3,
    host_read_model: HostBindingAuthorityV2ReadModel,
    candidate_records: Mapping[str, Mapping[str, object]],
) -> ContextApplicationV3HostBindingResult:
    if not isinstance(host_read_model, HostBindingAuthorityV2ReadModel):
        raise ContextApplicationV3HostBindingError(
            "HOST_AUTHORITY_INVALID", "host_binding_authority_v2"
        )
    if link.application_semantic_id != record.application_id:
        raise ContextApplicationV3HostBindingError(
            "APPLICATION_HOST_BINDING_UNKNOWN_APPLICATION", "application_semantic_id"
        )
    required = required_member_keys(record, candidate_records)
    if not required:
        raise ContextApplicationV3HostBindingError(
            "HOST_AUTHORITY_BINDING_UNEXPECTED", "application_host_bindings_v3"
        )
    expected = {encode_canonical(item.to_cbor()): item for item in required}
    admitted_claims_by_id = dict(host_read_model.admitted_claims_by_id)
    current_claims_by_id = dict(host_read_model.current_claims_by_id)
    seen: set[bytes] = set()
    for claim_id in link.host_binding_claim_ids:
        claim = admitted_claims_by_id.get(claim_id)
        if claim is None:
            raise ContextApplicationV3HostBindingError(
                "HOST_CLAIM_UNKNOWN", "host_binding_claim_ids"
            )
        if current_claims_by_id.get(claim_id) != claim:
            raise ContextApplicationV3HostBindingError(
                "HOST_CLAIM_NOT_CURRENT", "host_binding_claim_ids"
            )
        key = encode_canonical(claim.member_key.to_cbor())
        if key in seen or key not in expected:
            raise ContextApplicationV3HostBindingError(
                "HOST_MEMBER_SET_MISMATCH", "host_binding_claim_ids"
            )
        expected_relationship = record.members[
            next(
                index
                for index, member in enumerate(record.members)
                if member.candidate_id == claim.member_key.candidate_id
                and member.candidate_identity_digest_reference.digest_bytes
                == claim.member_key.candidate_identity_digest
                and member.source_instance_id == claim.member_key.source_instance_id
            )
        ].reviewed_context_binding_v1[3]
        if claim.observed_host_relationship != expected_relationship:
            raise ContextApplicationV3HostBindingError(
                "HOST_RELATIONSHIP_MISMATCH", "host_binding_claim_ids"
            )
        seen.add(key)
    if seen != set(expected):
        raise ContextApplicationV3HostBindingError(
            "HOST_MEMBER_SET_MISMATCH", "host_binding_claim_ids"
        )
    return ContextApplicationV3HostBindingResult(
        record.application_id.as_text(),
        link.host_binding_claim_ids,
        required,
    )


def admit_host_binding_authority_v2(
    source_resolver: AuthoritySourceResolver,
    binding: ContextAuthoritySourceBindingV3,
) -> HostBindingAuthorityV2ReadModel:
    """Admit the existing ADR-0044 authority before composing a V3 link."""

    if binding.artifact_role != "host_binding_authority_v2":
        raise ContextApplicationV3HostBindingError(
            "HOST_AUTHORITY_INVALID", "host_binding_authority_v2_binding"
        )
    try:
        artifact = source_resolver.resolve_repository_artifact(
            binding.path,
            binding.raw_sha256,
            binding.schema,
        )
        return AuthorityV2Validator(source_resolver).admit(artifact.json_value).read_model
    except (AuthorityV2ValidationError, ResolutionError) as exc:
        raise ContextApplicationV3HostBindingError(
            getattr(exc, "code", "HOST_AUTHORITY_INVALID"),
            "host_binding_authority_v2_binding",
        ) from exc


__all__ = [
    "ContextApplicationV3HostBindingError",
    "ContextApplicationV3HostBindingResult",
    "admit_host_binding_authority_v2",
    "required_member_keys",
    "validate_application_host_binding_v3",
]
