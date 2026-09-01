"""Fail-closed structural closure checks for the M2.5.C V2 underlay.

The V2 layer links existing V1 semantic Applications to member-atomic host
claims.  It does not redefine V1 identities or derive any C semantics.
"""

from __future__ import annotations

import io
import zipfile
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from types import MappingProxyType
from typing import cast

from authority_host_binding import HostBindingSourceResolver
from authority_source_resolver import AuthoritySourceResolver, B2ArtifactBindingsV1
from authority_validator import AuthorityValidator
from mtgml.authority import SourceBindingDigestV1
from mtgml.host_binding import (
    HOST_BINDING_ACCEPTANCE_EVENT_SCHEMA_V2,
    HOST_BINDING_AUTHORITY_SCHEMA_V2,
    ApplicationHostBindingV1,
    ApplicationMemberKeyV1,
    CrossDeckHostBindingClaimSupersessionV1,
    CrossDeckHostBindingClaimV1,
    HostBindingAcceptanceEventInputV2,
    HostBindingAcceptanceEventRefV2,
    HostBindingContractError,
    HostBindingEvidenceRefV2,
    HostBindingSourceBindingV2,
    application_host_binding_from_wire,
    host_binding_acceptance_event_from_wire,
    host_binding_claim_record_from_wire,
    host_binding_claim_supersession_from_wire,
    host_binding_source_binding_from_wire,
)
from mtgml.persistence import encode_canonical


class AuthorityV2ValidationError(ValueError):
    """Raised when V2 host-binding closure is structurally invalid."""


@dataclass(frozen=True)
class AuthorityV2ValidationResult:
    valid: bool
    counts: Mapping[str, int]


def _member_key_bytes(member: ApplicationMemberKeyV1) -> bytes:
    return encode_canonical(member.to_cbor())


def _identity_text(value: object, prefix: str) -> str:
    if not isinstance(value, Mapping):
        raise AuthorityV2ValidationError("V1 identity reference is not an object")
    digest = value.get("digest_hex")
    if not isinstance(digest, str) or len(digest) != 64:
        raise AuthorityV2ValidationError("V1 identity reference has an invalid digest")
    if any(character not in "0123456789abcdef" for character in digest):
        raise AuthorityV2ValidationError("V1 identity digest is not lowercase hexadecimal")
    return prefix + digest


def _v1_member_key(value: object) -> ApplicationMemberKeyV1:
    if not isinstance(value, Mapping):
        raise AuthorityV2ValidationError("V1 application member is not an object")
    identity = value.get("candidate_identity")
    if not isinstance(identity, Mapping):
        raise AuthorityV2ValidationError("V1 application member lacks candidate identity")
    digest = identity.get("digest_hex")
    if not isinstance(digest, str) or len(digest) != 64:
        raise AuthorityV2ValidationError("V1 application member identity is invalid")
    try:
        candidate_digest = bytes.fromhex(digest)
    except ValueError as exc:
        raise AuthorityV2ValidationError(
            "V1 application member identity is not hexadecimal"
        ) from exc
    return ApplicationMemberKeyV1(
        candidate_id=_v1_text(value.get("candidate_id"), "V1 candidate ID"),
        candidate_identity_digest=candidate_digest,
        source_instance_id=_v1_text(value.get("source_instance_id"), "V1 source instance ID"),
    )


def _v1_text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise AuthorityV2ValidationError(f"{label} is not non-empty text")
    return value


def _record_prefix(value: object) -> str:
    prefixes = {
        "relation_theorem_record": "rpr.v1/",
        "relation_application_record": "rpar.v1/",
        "domain_theorem_record": "dpr.v1/",
        "domain_application_record": "dpar.v1/",
        "context_theorem_record": "cpr.v1/",
        "context_application_record": "cpar.v1/",
    }
    prefix = prefixes.get(value)
    if prefix is None:
        raise AuthorityV2ValidationError("V1 supersession record kind is not a known V1 kind")
    return prefix


def _host_expectation(theorem: Mapping[str, object], applications_key: str) -> str | None:
    if applications_key == "relation_applications":
        subject = theorem.get("subject")
        if not isinstance(subject, Mapping):
            raise AuthorityV2ValidationError("V1 relation theorem subject is malformed")
        return _v1_text(subject.get("host_relationship"), "V1 relation host relationship")
    if applications_key == "context_applications":
        subject = theorem.get("subject_shape")
        if not isinstance(subject, Mapping):
            raise AuthorityV2ValidationError("V1 context theorem subject is malformed")
        return _v1_text(subject.get("host_relationship"), "V1 context host relationship")
    if applications_key == "domain_applications":
        raw_preconditions = theorem.get("preconditions")
        if not isinstance(raw_preconditions, list):
            raise AuthorityV2ValidationError("V1 domain theorem preconditions are malformed")
        candidates: list[str] = []
        for item in raw_preconditions:
            if not isinstance(item, Mapping) or item.get("precondition_kind") != (
                "candidate_relation_shape"
            ):
                continue
            payload = item.get("payload")
            if not isinstance(payload, list) or len(payload) != 4:
                raise AuthorityV2ValidationError("V1 candidate relation precondition is malformed")
            candidates.append(_v1_text(payload[3], "V1 domain host relationship"))
        if not candidates:
            return None
        if len(set(candidates)) != 1:
            raise AuthorityV2ValidationError("V1 domain host expectations are ambiguous")
        return candidates[0]
    raise AuthorityV2ValidationError("unknown V1 application family")


def validate_application_host_closure(
    claims: Sequence[CrossDeckHostBindingClaimV1],
    application_links: Sequence[ApplicationHostBindingV1],
    application_members: Mapping[str, Sequence[ApplicationMemberKeyV1]],
    expected_host_relationships: Mapping[str, str],
    required_host_members: Mapping[str, Sequence[ApplicationMemberKeyV1]] | None = None,
) -> None:
    """Validate member coverage and cross-layer host expectations.

    ``application_members`` and ``expected_host_relationships`` are supplied
    by the exact V1 authority graph.  The host-binding layer never derives
    either value from capability names, discovery order, or application
    partitioning.
    """

    claims_by_id: dict[str, CrossDeckHostBindingClaimV1] = {}
    claim_by_member: dict[bytes, str] = {}
    for claim in claims:
        if not isinstance(claim, CrossDeckHostBindingClaimV1):
            raise AuthorityV2ValidationError("host claim is not a V1 claim")
        claim_id = claim.identity().as_text()
        if claim_id in claims_by_id:
            raise AuthorityV2ValidationError("host claim identity is duplicated")
        claims_by_id[claim_id] = claim
        member_key = _member_key_bytes(claim.member_key)
        previous = claim_by_member.get(member_key)
        if previous is not None and previous != claim_id:
            raise AuthorityV2ValidationError(
                "one application member is bound to multiple current host claims"
            )
        claim_by_member[member_key] = claim_id

    links_by_application: dict[str, ApplicationHostBindingV1] = {}
    for link in application_links:
        application_id = link.application_semantic_id
        if application_id in links_by_application:
            raise AuthorityV2ValidationError(
                f"application {application_id!r} has duplicate host-binding links"
            )
        if application_id not in application_members:
            raise AuthorityV2ValidationError(
                f"application {application_id!r} is absent from the V1 authority graph"
            )
        links_by_application[application_id] = link

    required_members_by_application = (
        {application_id: tuple(members) for application_id, members in application_members.items()}
        if required_host_members is None
        else {
            application_id: tuple(members)
            for application_id, members in required_host_members.items()
        }
    )
    required_application_ids = set(required_members_by_application)
    if not required_application_ids.issubset(application_members):
        unknown = sorted(required_application_ids - set(application_members))
        raise AuthorityV2ValidationError(
            f"V2 host-binding requirement names unknown applications: {unknown!r}"
        )
    if set(links_by_application) != required_application_ids:
        missing = sorted(required_application_ids - set(links_by_application))
        extra = sorted(set(links_by_application) - required_application_ids)
        raise AuthorityV2ValidationError(
            f"V2 host-binding links do not close the required V1 application set; "
            f"missing={missing!r}, extra={extra!r}"
        )

    for application_id in required_application_ids:
        members = required_members_by_application[application_id]
        if not members:
            raise AuthorityV2ValidationError(
                f"application {application_id!r} has an empty required host member set"
            )
        link = links_by_application[application_id]
        expected_relationship = expected_host_relationships.get(application_id)

        member_keys = [_member_key_bytes(member) for member in members]
        if len(set(member_keys)) != len(member_keys):
            raise AuthorityV2ValidationError(
                f"application {application_id!r} contains duplicate members"
            )

        linked_claims: list[CrossDeckHostBindingClaimV1] = []
        linked_member_keys: list[bytes] = []
        for claim_id in link.host_binding_claim_ids:
            claim = claims_by_id.get(claim_id)
            if claim is None:
                raise AuthorityV2ValidationError(
                    f"application {application_id!r} references an unknown host claim"
                )
            linked_claims.append(claim)
            linked_member_keys.append(_member_key_bytes(claim.member_key))
            if (
                expected_relationship is not None
                and claim.observed_host_relationship != expected_relationship
            ):
                raise AuthorityV2ValidationError(
                    f"host claim {claim_id!r} differs from the theorem host expectation"
                )

        if len(set(linked_member_keys)) != len(linked_member_keys):
            raise AuthorityV2ValidationError(
                f"application {application_id!r} links a member more than once"
            )
        if set(linked_member_keys) != set(member_keys):
            raise AuthorityV2ValidationError(
                f"application {application_id!r} claim union differs from its members"
            )

        for claim, member_key in zip(linked_claims, linked_member_keys, strict=True):
            current_claim_id = claim_by_member.get(member_key)
            if current_claim_id != claim.identity().as_text():
                raise AuthorityV2ValidationError(
                    f"application {application_id!r} does not use the current claim for a member"
                )


class AuthorityV2Validator:
    """Validate the V2 host-binding envelope around an exact V1 authority."""

    def __init__(self, resolver: AuthoritySourceResolver) -> None:
        self._resolver = resolver
        self._host_resolver = HostBindingSourceResolver(resolver)

    def validate(self, value: object) -> AuthorityV2ValidationResult:
        try:
            return self._validate_document(value)
        except HostBindingContractError as exc:
            raise AuthorityV2ValidationError(str(exc)) from exc

    def _validate_document(self, value: object) -> AuthorityV2ValidationResult:
        document = self._exact_object(
            value,
            {
                "schema",
                "base_authority_v1_binding",
                "source_bindings",
                "cross_deck_host_binding_claim_records",
                "cross_deck_host_binding_claim_supersession_records",
                "application_host_bindings",
            },
            "V2 authority document",
        )
        if document["schema"] != HOST_BINDING_AUTHORITY_SCHEMA_V2:
            raise AuthorityV2ValidationError("authority document is not the V2 contract")

        raw_sources = document["source_bindings"]
        if not isinstance(raw_sources, list):
            raise AuthorityV2ValidationError("V2 source_bindings must be an array")
        source_bindings = tuple(host_binding_source_binding_from_wire(item) for item in raw_sources)
        source_bytes = [encode_canonical(binding.to_cbor()) for binding in source_bindings]
        if source_bytes != sorted(source_bytes) or len(set(source_bytes)) != len(source_bytes):
            raise AuthorityV2ValidationError("V2 source_bindings must be canonical and unique")
        b2_source_bindings = self._b2_source_bindings(source_bindings)
        b2_bindings = self._b2_bindings_from_source_bindings(b2_source_bindings)
        self._host_resolver = HostBindingSourceResolver(
            self._resolver,
            b2_bindings=b2_bindings,
        )

        base_binding = host_binding_source_binding_from_wire(document["base_authority_v1_binding"])
        if base_binding.artifact_role != "base_authority_v1":
            raise AuthorityV2ValidationError("V2 base binding has the wrong source role")
        if base_binding not in source_bindings:
            raise AuthorityV2ValidationError("V2 base binding is absent from source_bindings")
        base_artifact = self._resolver.resolve_repository_artifact(
            base_binding.path,
            base_binding.raw_sha256,
            base_binding.schema_or_null,
        )
        base_document = base_artifact.json_value
        if not isinstance(base_document, Mapping):
            raise AuthorityV2ValidationError("V2 base authority is not a JSON object")
        AuthorityValidator(self._resolver).validate(dict(base_document))

        raw_records = document["cross_deck_host_binding_claim_records"]
        if not isinstance(raw_records, list):
            raise AuthorityV2ValidationError("V2 claim records must be an array")
        claim_records = tuple(host_binding_claim_record_from_wire(item) for item in raw_records)
        claims = tuple(record.claim for record in claim_records)
        record_ids = [record.record_identity().as_text() for record in claim_records]
        if len(set(record_ids)) != len(record_ids):
            raise AuthorityV2ValidationError("V2 claim record identities must be unique")
        claim_ids = [claim.identity().as_text() for claim in claims]
        if claim_records and b2_bindings is None:
            raise AuthorityV2ValidationError(
                "V2 host-binding claims require the complete B2 catalog/classification/closure set"
            )
        model_bindings = [
            binding for binding in source_bindings if binding.artifact_role == "declared_model"
        ]
        raw_supersessions = document["cross_deck_host_binding_claim_supersession_records"]
        if not isinstance(raw_supersessions, list):
            raise AuthorityV2ValidationError("V2 claim supersessions must be an array")
        if (claim_records or raw_supersessions) and len(model_bindings) != 1:
            raise AuthorityV2ValidationError(
                "V2 accepted host-binding records require exactly one declared-model binding"
            )
        model_binding = model_bindings[0] if model_bindings else None
        candidate_bindings = [
            binding for binding in source_bindings if binding.artifact_role == "candidate_universe"
        ]
        pair_bindings = [
            binding
            for binding in source_bindings
            if binding.artifact_role == "rev3_pair_aggregates"
        ]
        if claim_records and len(candidate_bindings) != 1:
            raise AuthorityV2ValidationError(
                "V2 host-binding claims require exactly one candidate-universe binding"
            )
        if claim_records and len(pair_bindings) != 1:
            raise AuthorityV2ValidationError(
                "V2 host-binding claims require exactly one REV3 pair-aggregate binding"
            )
        candidate_binding = candidate_bindings[0] if candidate_bindings else None
        pair_binding = pair_bindings[0] if pair_bindings else None
        used_bindings = {
            encode_canonical(base_binding.to_cbor()),
        }
        for claim in claims:
            if candidate_binding is None or pair_binding is None:
                raise AuthorityV2ValidationError(
                    "V2 host-binding claim lacks its candidate or pair source binding"
                )
            self._host_resolver.resolve_claim_for_member(
                claim,
                claim.member_key.candidate_identity_digest,
                candidate_binding,
                pair_binding,
            )

        for record in claim_records:
            event = self._validate_acceptance_event(
                record.acceptance_event_ref,
                "cross_deck_host_binding_claim_record_v1",
                record.claim.identity().digest_bytes,
            )
            if model_binding is None:
                raise AuthorityV2ValidationError("V2 claim event lacks its model binding")
            expected_event_sources = self._expected_acceptance_sources(
                event,
                model_binding,
                self._claim_evidence(record.claim),
                b2_bindings,
                candidate_binding,
                pair_binding,
            )
            actual_event_sources = {
                encode_canonical(binding.to_cbor()) for binding in event.source_binding_digests
            }
            if actual_event_sources != expected_event_sources:
                raise AuthorityV2ValidationError(
                    "V2 claim acceptance source bindings are not the exact claim closure"
                )
            used_bindings.update(actual_event_sources)
            used_bindings.add(
                encode_canonical(
                    HostBindingSourceBindingV2(
                        "acceptance_event_leaf_v2",
                        record.acceptance_event_ref.path,
                        HOST_BINDING_ACCEPTANCE_EVENT_SCHEMA_V2,
                        record.acceptance_event_ref.raw_sha256,
                    ).to_cbor()
                )
            )

        raw_links = document["application_host_bindings"]
        if not isinstance(raw_links, list):
            raise AuthorityV2ValidationError("V2 application_host_bindings must be an array")
        links = tuple(application_host_binding_from_wire(item) for item in raw_links)
        for link in links:
            for claim_id in link.host_binding_claim_ids:
                if claim_id not in claim_ids:
                    raise AuthorityV2ValidationError(
                        f"application link references unknown claim {claim_id!r}"
                    )

        supersessions: list[CrossDeckHostBindingClaimSupersessionV1] = []
        superseded_record_ids: set[str] = set()
        known_record_ids = {record.record_identity().as_text() for record in claim_records}
        successor_by_record: dict[str, str | None] = {}
        for item in raw_supersessions:
            supersession = self._parse_supersession(item)
            superseded = supersession.superseded_record_id.as_text()
            replacement = (
                None
                if supersession.replacement_record_id is None
                else supersession.replacement_record_id.as_text()
            )
            if superseded in successor_by_record:
                raise AuthorityV2ValidationError("a V2 claim record has multiple successors")
            if superseded not in known_record_ids:
                raise AuthorityV2ValidationError("V2 supersession targets an unknown record")
            if replacement is not None and replacement not in known_record_ids:
                raise AuthorityV2ValidationError("V2 supersession replacement is unknown")
            successor_by_record[superseded] = replacement
            superseded_record_ids.add(superseded)
            supersessions.append(supersession)
            for evidence in supersession.source_evidence_refs:
                self._host_resolver.resolve_evidence_reference(evidence)
                used_bindings.add(encode_canonical(self._binding_from_evidence(evidence).to_cbor()))
            event = self._validate_acceptance_event(
                supersession.acceptance_event_ref,
                "cross_deck_host_binding_claim_supersession_v1",
                supersession.identity().digest_bytes,
            )
            if model_binding is None:
                raise AuthorityV2ValidationError("V2 supersession event lacks its model binding")
            supersession_uses_b2 = any(
                getattr(evidence, "artifact_role", None)
                in {"b2_catalog", "b2_classifications", "b2_closure"}
                for evidence in supersession.source_evidence_refs
            )
            expected_event_sources = self._expected_acceptance_sources(
                event,
                model_binding,
                supersession.source_evidence_refs,
                b2_bindings if supersession_uses_b2 else None,
                available_b2_bindings=b2_source_bindings if supersession_uses_b2 else None,
            )
            actual_event_sources = {
                encode_canonical(binding.to_cbor()) for binding in event.source_binding_digests
            }
            if actual_event_sources != expected_event_sources:
                raise AuthorityV2ValidationError(
                    "V2 supersession acceptance source bindings are not the exact closure"
                )
            used_bindings.update(actual_event_sources)
            used_bindings.add(
                encode_canonical(
                    HostBindingSourceBindingV2(
                        "acceptance_event_leaf_v2",
                        supersession.acceptance_event_ref.path,
                        HOST_BINDING_ACCEPTANCE_EVENT_SCHEMA_V2,
                        supersession.acceptance_event_ref.raw_sha256,
                    ).to_cbor()
                )
            )

        for start in successor_by_record:
            seen: set[str] = set()
            current = start
            while current in successor_by_record:
                if current in seen:
                    raise AuthorityV2ValidationError("V2 claim supersession graph contains a cycle")
                seen.add(current)
                next_record = successor_by_record[current]
                if next_record is None:
                    break
                current = next_record

        current_records = tuple(
            record
            for record in claim_records
            if record.record_identity().as_text() not in superseded_record_ids
        )
        current_claim_by_id: dict[str, CrossDeckHostBindingClaimV1] = {}
        for record in current_records:
            claim_id = record.claim.identity().as_text()
            if claim_id in current_claim_by_id:
                raise AuthorityV2ValidationError(
                    "multiple current record revisions exist for one host-binding claim"
                )
            current_claim_by_id[claim_id] = record.claim
        current_claims = tuple(current_claim_by_id.values())
        application_members, expected_hosts, member_sources = self._v1_application_facts(
            dict(base_document),
            superseded_record_ids,
        )
        validate_application_host_closure(
            current_claims,
            links,
            application_members,
            expected_hosts,
            self._host_binding_application_members(application_members, member_sources),
        )
        for link in links:
            if pair_binding is not None:
                used_bindings.add(encode_canonical(pair_binding.to_cbor()))
            for claim_id in link.host_binding_claim_ids:
                claim = next(
                    (
                        candidate
                        for candidate in current_claims
                        if candidate.identity().as_text() == claim_id
                    ),
                    None,
                )
                if claim is None:
                    raise AuthorityV2ValidationError("application link lacks a current claim")
                member_source = member_sources.get(_member_key_bytes(claim.member_key))
                if member_source is None or pair_binding is None:
                    raise AuthorityV2ValidationError(
                        "linked host claim lacks candidate or pair source binding"
                    )
                _, member_candidate_binding = member_source
                expected_candidate_binding = HostBindingSourceBindingV2(
                    "candidate_universe",
                    member_candidate_binding.path,
                    member_candidate_binding.schema_or_null,
                    member_candidate_binding.raw_sha256,
                )
                if candidate_binding != expected_candidate_binding:
                    raise AuthorityV2ValidationError(
                        "V1 application member uses another candidate-universe snapshot"
                    )

        actual_bindings = {encode_canonical(binding.to_cbor()) for binding in source_bindings}
        if actual_bindings != used_bindings:
            raise AuthorityV2ValidationError(
                "V2 source_bindings are not the exact used binding set"
            )

        return AuthorityV2ValidationResult(
            valid=True,
            counts=MappingProxyType(
                {
                    "cross_deck_host_binding_claim_records": len(claim_records),
                    "application_host_bindings": len(links),
                    "cross_deck_host_binding_claim_supersession_records": len(raw_supersessions),
                }
            ),
        )

    def _validate_acceptance_event(
        self,
        reference: HostBindingAcceptanceEventRefV2,
        expected_subject_kind: str,
        expected_subject_digest: bytes,
    ) -> HostBindingAcceptanceEventInputV2:
        artifact = self._resolver.resolve_repository_artifact(
            reference.path,
            reference.raw_sha256,
            HOST_BINDING_ACCEPTANCE_EVENT_SCHEMA_V2,
        )
        value = artifact.json_value
        if not isinstance(value, Mapping):
            raise AuthorityV2ValidationError("V2 acceptance event is not a JSON object")
        try:
            event = host_binding_acceptance_event_from_wire(dict(value))
        except HostBindingContractError as exc:
            raise AuthorityV2ValidationError(str(exc)) from exc
        if event.subject_kind != expected_subject_kind:
            raise AuthorityV2ValidationError("V2 acceptance subject kind differs from its record")
        if event.subject_payload_digest != expected_subject_digest:
            raise AuthorityV2ValidationError("V2 acceptance subject digest differs from its record")
        if event.identity().as_text() != reference.event_id:
            raise AuthorityV2ValidationError("V2 acceptance event ID differs from its leaf path")

        roster_artifact = self._resolver.resolve_reviewer_roster_leaf(event.reviewer_roster_ref)
        roster_value = roster_artifact.json_value
        if not isinstance(roster_value, Mapping):
            raise AuthorityV2ValidationError("V2 reviewer roster is not a JSON object")
        roster_reviewers = roster_value.get("reviewers")
        if not isinstance(roster_reviewers, list):
            raise AuthorityV2ValidationError("V2 reviewer roster reviewers are not an array")
        roster_roles: dict[str, tuple[str, ...]] = {}
        for item in roster_reviewers:
            if not isinstance(item, Mapping):
                raise AuthorityV2ValidationError("V2 reviewer roster entry is not an object")
            reviewer_id = item.get("reviewer_id")
            roles = item.get("roles")
            if not isinstance(reviewer_id, str) or not isinstance(roles, list):
                raise AuthorityV2ValidationError("V2 reviewer roster entry is malformed")
            if reviewer_id in roster_roles or any(not isinstance(role, str) for role in roles):
                raise AuthorityV2ValidationError("V2 reviewer roster entries are ambiguous")
            roster_roles[reviewer_id] = tuple(cast(list[str], roles))
        event_roles = {
            binding.reviewer_id: binding.roles for binding in event.reviewer_role_bindings
        }
        if event_roles != roster_roles:
            raise AuthorityV2ValidationError(
                "V2 reviewer role bindings differ from the bound roster"
            )
        if "architecture_maintainer" not in {
            role for roles in event_roles.values() for role in roles
        }:
            raise AuthorityV2ValidationError(
                "V2 cross-artifact acceptance requires architecture_maintainer"
            )

        expected_roster_binding = HostBindingSourceBindingV2(
            "reviewer_roster_leaf",
            event.reviewer_roster_ref.path,
            event.reviewer_roster_ref.schema,
            event.reviewer_roster_ref.raw_sha256,
        )
        if expected_roster_binding not in event.source_binding_digests:
            raise AuthorityV2ValidationError("V2 acceptance does not bind its reviewer roster")
        for binding in event.source_binding_digests:
            if binding.artifact_role == "acceptance_event_leaf_v2":
                raise AuthorityV2ValidationError("V2 acceptance event cannot bind its own leaf")
            self._resolve_v2_source_binding(binding)
        for evidence in event.review_evidence_refs:
            self._resolve_acceptance_evidence(evidence)
        return event

    @staticmethod
    def _expected_acceptance_sources(
        event: HostBindingAcceptanceEventInputV2,
        model_binding: HostBindingSourceBindingV2,
        evidence: Sequence[object],
        b2_bindings: B2ArtifactBindingsV1 | None,
        candidate_binding: HostBindingSourceBindingV2 | None = None,
        pair_binding: HostBindingSourceBindingV2 | None = None,
        available_b2_bindings: Mapping[str, HostBindingSourceBindingV2] | None = None,
    ) -> set[bytes]:
        expected = {
            encode_canonical(model_binding.to_cbor()),
            encode_canonical(
                HostBindingSourceBindingV2(
                    "reviewer_roster_leaf",
                    event.reviewer_roster_ref.path,
                    event.reviewer_roster_ref.schema,
                    event.reviewer_roster_ref.raw_sha256,
                ).to_cbor()
            ),
        }
        expected.update(
            encode_canonical(AuthorityV2Validator._binding_from_evidence(reference).to_cbor())
            for reference in evidence
        )
        if candidate_binding is not None:
            expected.add(encode_canonical(candidate_binding.to_cbor()))
        if pair_binding is not None:
            expected.add(encode_canonical(pair_binding.to_cbor()))
        if b2_bindings is not None or available_b2_bindings is not None:
            source_bindings = (
                available_b2_bindings
                if available_b2_bindings is not None
                else AuthorityV2Validator._b2_source_bindings_from_full(b2_bindings)
            )
            expected.update(
                encode_canonical(binding.to_cbor())
                for binding in AuthorityV2Validator._b2_acceptance_bindings(
                    evidence, source_bindings
                )
            )
        return expected

    @staticmethod
    def _b2_acceptance_bindings(
        evidence: Sequence[object],
        available_b2_bindings: Mapping[str, HostBindingSourceBindingV2],
    ) -> tuple[HostBindingSourceBindingV2, ...]:
        """Expand B2 evidence to the exact source closure it requires."""

        roles = {getattr(reference, "artifact_role", None) for reference in evidence}
        if "b2_classifications" in roles:
            required_roles = ("b2_catalog", "b2_classifications", "b2_closure")
        elif "b2_catalog" in roles:
            required_roles = ("b2_catalog", "b2_closure")
        elif "b2_closure" in roles:
            required_roles = ("b2_closure",)
        else:
            return ()

        required_bindings = {
            role: available_b2_bindings[role]
            for role in required_roles
            if role in available_b2_bindings
        }
        missing = [role for role in required_roles if role not in required_bindings]
        if missing:
            raise AuthorityV2ValidationError(
                f"B2 evidence requires missing source bindings: {missing!r}"
            )
        return tuple(required_bindings[role] for role in required_roles)

    def _host_binding_application_members(
        self,
        application_members: Mapping[str, Sequence[ApplicationMemberKeyV1]],
        member_sources: Mapping[bytes, tuple[Mapping[str, object], SourceBindingDigestV1]],
    ) -> dict[str, tuple[ApplicationMemberKeyV1, ...]]:
        required: dict[str, tuple[ApplicationMemberKeyV1, ...]] = {}
        for application_id, members in application_members.items():
            required_members: list[ApplicationMemberKeyV1] = []
            for member in members:
                source = member_sources.get(_member_key_bytes(member))
                if source is None:
                    raise AuthorityV2ValidationError(
                        f"application {application_id!r} lacks a candidate source binding"
                    )
                identity, binding = source
                candidate = self._resolver.resolve_candidate(
                    member.candidate_id,
                    identity,
                    binding,
                )
                record = candidate.candidate_record
                if (
                    record.get("scope") == "cross_deck"
                    and record.get("relation") == "directional_binary"
                ):
                    required_members.append(member)
            if required_members:
                required[application_id] = tuple(required_members)
        return required

    def _resolve_v2_source_binding(self, binding: HostBindingSourceBindingV2) -> None:
        if binding.artifact_role.startswith("rev3_"):
            self._resolver.resolve_rev3_member(
                binding.path,
                binding.raw_sha256,
                binding.schema_or_null,
            )
        else:
            self._resolver.resolve_repository_artifact(
                binding.path,
                binding.raw_sha256,
                binding.schema_or_null,
            )

    def _resolve_acceptance_evidence(self, evidence: object) -> None:
        path = cast(str, evidence.path)
        artifact = self._resolver.resolve_repository_artifact(path, evidence.raw_sha256, None)
        kind, payload = evidence.locator
        if kind == "whole_artifact":
            return
        if kind == "json_pointer":
            self._resolver.resolve_locator(artifact, ("json_pointer", cast(str, payload)))
            return
        if kind == "archive_member":
            try:
                with zipfile.ZipFile(io.BytesIO(artifact.raw_bytes)) as archive:
                    member = cast(str, payload)
                    if member not in archive.namelist():
                        raise AuthorityV2ValidationError("V2 evidence archive member is missing")
                    archive.read(member)
            except (OSError, zipfile.BadZipFile) as exc:
                raise AuthorityV2ValidationError(
                    "V2 evidence archive member is unreadable"
                ) from exc
            return
        raise AuthorityV2ValidationError("V2 acceptance evidence locator is unsupported")

    @staticmethod
    def _binding_from_evidence(
        reference: object,
    ) -> HostBindingSourceBindingV2:
        typed = cast("HostBindingEvidenceRefV2", reference)
        return HostBindingSourceBindingV2(
            typed.artifact_role,
            typed.path,
            typed.schema_or_null,
            typed.raw_sha256,
        )

    @staticmethod
    def _claim_evidence(
        claim: CrossDeckHostBindingClaimV1,
    ) -> tuple[object, ...]:
        result: list[object] = []
        for binding in claim.discovery_bindings:
            result.extend(binding.mapping_evidence_refs)
        for realization in claim.participant_host_realizations:
            for witness in realization.witnesses:
                result.extend(
                    (
                        witness.discovery_mapping_ref,
                        witness.deck_row_ref,
                        witness.osi_ref,
                        *witness.b2_assignment_refs,
                    )
                )
        encoded = {
            encode_canonical(cast(list[object], reference.to_cbor())): reference
            for reference in result
        }
        return tuple(encoded.values())

    @staticmethod
    def _parse_supersession(
        value: object,
    ) -> CrossDeckHostBindingClaimSupersessionV1:
        try:
            return host_binding_claim_supersession_from_wire(value)
        except HostBindingContractError as exc:
            raise AuthorityV2ValidationError(str(exc)) from exc

    @staticmethod
    def _v1_application_facts(
        document: Mapping[str, object],
        v2_superseded_record_ids: set[str],
    ) -> tuple[
        dict[str, tuple[ApplicationMemberKeyV1, ...]],
        dict[str, str],
        dict[bytes, tuple[Mapping[str, object], SourceBindingDigestV1]],
    ]:
        application_specs = (
            (
                "relation_applications",
                "relation_proofs",
                "rpa.v1/",
                "rpar.v1/",
                "rpr.v1/",
                "relation theorem",
            ),
            (
                "domain_applications",
                "domain_proofs",
                "dpa.v1/",
                "dpar.v1/",
                "dpr.v1/",
                "domain theorem",
            ),
            (
                "context_applications",
                "context_proofs",
                "cpa.v1/",
                "cpar.v1/",
                "cpr.v1/",
                "context theorem",
            ),
        )
        v1_superseded = set(v2_superseded_record_ids)
        raw_v1_supersessions = document.get("supersession_records")
        if isinstance(raw_v1_supersessions, list):
            for item in raw_v1_supersessions:
                if isinstance(item, Mapping):
                    try:
                        v1_superseded.add(
                            _identity_text(
                                item.get("superseded_record_id"),
                                _record_prefix(item.get("superseded_record_kind")),
                            )
                        )
                    except AuthorityV2ValidationError:
                        continue

        application_members: dict[str, tuple[ApplicationMemberKeyV1, ...]] = {}
        expected_hosts: dict[str, str] = {}
        member_sources: dict[bytes, tuple[Mapping[str, object], SourceBindingDigestV1]] = {}
        for (
            applications_key,
            theorems_key,
            application_prefix,
            record_prefix,
            theorem_record_prefix,
            theorem_label,
        ) in application_specs:
            raw_theorems = document.get(theorems_key)
            raw_applications = document.get(applications_key)
            if not isinstance(raw_theorems, list) or not isinstance(raw_applications, list):
                raise AuthorityV2ValidationError(f"V1 {theorem_label} arrays are malformed")
            theorem_by_record_id: dict[str, Mapping[str, object]] = {}
            for item in raw_theorems:
                if not isinstance(item, Mapping):
                    raise AuthorityV2ValidationError(f"V1 {theorem_label} is not an object")
                record_id = _identity_text(item.get("record_id"), theorem_record_prefix)
                if record_id in theorem_by_record_id:
                    raise AuthorityV2ValidationError(f"duplicate V1 {theorem_label} record")
                theorem_by_record_id[record_id] = item

            for item in raw_applications:
                if not isinstance(item, Mapping):
                    raise AuthorityV2ValidationError("V1 application record is not an object")
                application_id = _identity_text(item.get("application_id"), application_prefix)
                record_id = _identity_text(item.get("record_id"), record_prefix)
                if record_id in v1_superseded:
                    continue
                if application_id in application_members:
                    raise AuthorityV2ValidationError(
                        f"multiple current V1 application records for {application_id!r}"
                    )
                raw_members = item.get("members")
                if not isinstance(raw_members, list):
                    raise AuthorityV2ValidationError(
                        f"V1 application {application_id!r} members are malformed"
                    )
                members = tuple(_v1_member_key(member) for member in raw_members)
                application_members[application_id] = members
                for raw_member, member_key in zip(raw_members, members, strict=True):
                    if not isinstance(raw_member, Mapping):
                        raise AuthorityV2ValidationError("V1 application member is not an object")
                    identity = raw_member.get("candidate_identity")
                    binding = raw_member.get("candidate_universe_binding")
                    if not isinstance(identity, Mapping) or not isinstance(binding, Mapping):
                        continue
                    try:
                        source_binding = SourceBindingDigestV1(
                            "candidate_universe",
                            _v1_text(binding.get("path"), "V1 candidate source path"),
                            _v1_text(
                                binding.get("schema"),
                                "V1 candidate source schema",
                            ),
                            bytes.fromhex(
                                _v1_text(binding.get("raw_sha256"), "V1 candidate source digest")
                            ),
                        )
                    except (TypeError, ValueError) as exc:
                        raise AuthorityV2ValidationError(
                            "V1 candidate source binding is malformed"
                        ) from exc
                    member_key_bytes = _member_key_bytes(member_key)
                    prior = member_sources.get(member_key_bytes)
                    current = (dict(identity), source_binding)
                    if prior is not None and prior != current:
                        raise AuthorityV2ValidationError(
                            "one member has conflicting V1 candidate source bindings"
                        )
                    member_sources[member_key_bytes] = current
                theorem_record_id = _identity_text(
                    item.get("theorem_record_id"), theorem_record_prefix
                )
                theorem = theorem_by_record_id.get(theorem_record_id)
                if theorem is None:
                    raise AuthorityV2ValidationError(
                        f"V1 application {application_id!r} theorem record is unknown"
                    )
                host = _host_expectation(theorem, applications_key)
                if host is not None:
                    expected_hosts[application_id] = host
        return application_members, expected_hosts, member_sources

    @staticmethod
    def _exact_object(value: object, expected_keys: set[str], label: str) -> dict[str, object]:
        if not isinstance(value, Mapping) or set(value) != expected_keys:
            raise AuthorityV2ValidationError(
                f"{label} fields are not exactly {sorted(expected_keys)!r}"
            )
        return {cast(str, key): item for key, item in value.items()}

    @staticmethod
    def _b2_source_bindings(
        source_bindings: Sequence[object],
    ) -> dict[str, HostBindingSourceBindingV2]:
        singleton_roles = {"b2_catalog", "b2_classifications", "b2_closure"}
        by_role: dict[str, HostBindingSourceBindingV2] = {}
        for binding in source_bindings:
            if not isinstance(binding, HostBindingSourceBindingV2):
                continue
            if binding.artifact_role not in singleton_roles:
                continue
            if binding.artifact_role in by_role:
                raise AuthorityV2ValidationError(
                    f"V2 source role {binding.artifact_role!r} is duplicated"
                )
            by_role[binding.artifact_role] = binding
        return by_role

    @staticmethod
    def _b2_bindings_from_source_bindings(
        by_role: Mapping[str, HostBindingSourceBindingV2],
    ) -> B2ArtifactBindingsV1 | None:
        if not {"b2_catalog", "b2_classifications", "b2_closure"}.issubset(by_role):
            return None
        try:
            return B2ArtifactBindingsV1(
                catalog=SourceBindingDigestV1(
                    "b2_catalog",
                    by_role["b2_catalog"].path,
                    by_role["b2_catalog"].schema_or_null,
                    by_role["b2_catalog"].raw_sha256,
                ),
                classifications=SourceBindingDigestV1(
                    "b2_classifications",
                    by_role["b2_classifications"].path,
                    by_role["b2_classifications"].schema_or_null,
                    by_role["b2_classifications"].raw_sha256,
                ),
                closure=SourceBindingDigestV1(
                    "b2_closure",
                    by_role["b2_closure"].path,
                    by_role["b2_closure"].schema_or_null,
                    by_role["b2_closure"].raw_sha256,
                ),
            )
        except (TypeError, ValueError) as exc:
            raise AuthorityV2ValidationError("V2 B2 source bindings are malformed") from exc

    @staticmethod
    def _b2_source_bindings_from_full(
        b2_bindings: B2ArtifactBindingsV1 | None,
    ) -> dict[str, HostBindingSourceBindingV2]:
        if b2_bindings is None:
            raise AuthorityV2ValidationError("complete B2 source bindings are required")
        return {
            "b2_catalog": HostBindingSourceBindingV2(
                "b2_catalog",
                b2_bindings.catalog.path,
                b2_bindings.catalog.schema_or_null,
                b2_bindings.catalog.raw_sha256,
            ),
            "b2_classifications": HostBindingSourceBindingV2(
                "b2_classifications",
                b2_bindings.classifications.path,
                b2_bindings.classifications.schema_or_null,
                b2_bindings.classifications.raw_sha256,
            ),
            "b2_closure": HostBindingSourceBindingV2(
                "b2_closure",
                b2_bindings.closure.path,
                b2_bindings.closure.schema_or_null,
                b2_bindings.closure.raw_sha256,
            ),
        }

    @staticmethod
    def _b2_bindings(
        source_bindings: Sequence[object],
    ) -> B2ArtifactBindingsV1 | None:
        return AuthorityV2Validator._b2_bindings_from_source_bindings(
            AuthorityV2Validator._b2_source_bindings(source_bindings)
        )


__all__ = [
    "AuthorityV2ValidationError",
    "AuthorityV2ValidationResult",
    "AuthorityV2Validator",
    "validate_application_host_closure",
]
