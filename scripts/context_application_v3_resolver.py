"""ContextApplicationV3 source, theorem, RPA, and V4 dependency resolution."""

from __future__ import annotations

import sys
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from authority_source_resolver import AuthoritySourceResolver, ResolutionError
from authority_validator import AuthorityValidator
from context_application_v2_resolver import (
    ContextApplicationV2Resolver,
    reconstruct_event_source_closure,
)
from mtgml.authority import (
    AcceptanceEvidenceRefV1,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationAuthorityV3,
    ContextApplicationMemberV2,
    ContextApplicationMemberV3,
    ContextApplicationV2Record,
    ContextApplicationV3Record,
    ContextAuthoritySourceBindingV2,
    DigestReferenceV1,
    RelationApplicationV2Record,
    ReviewAcceptanceEventLeafV4,
    ReviewAuthoritySourceBindingV4,
    ReviewerRosterRefV1,
    ReviewEventRefV1,
    SourceBindingDigestV1,
    V1DependencySourceBindingToV4,
)
from mtgml.persistence import encode_canonical
from relation_application_v2_resolver import RelationApplicationV2Resolver
from relation_application_v2_supersession import admit_relation_application_authority_v2


class ContextApplicationV3ResolutionError(ValueError):
    def __init__(self, code: str, location: str, *, cause_code: str | None = None) -> None:
        self.code = code
        self.location = location
        self.cause_code = cause_code
        super().__init__(f"{code} at {location}")


@dataclass(frozen=True)
class ResolvedRpaV2Member:
    record: RelationApplicationV2Record
    member: object


class ContextApplicationV3RpaMemberResolver(Protocol):
    def resolve_current_rpa_member(
        self,
        application_id: object,
        candidate_id: str,
        candidate_identity: DigestReferenceV1,
        source_instance_id: str,
    ) -> ResolvedRpaV2Member: ...

    def expected_rpa_source_closure(
        self, record: RelationApplicationV2Record, reviewer_roster_ref: ReviewerRosterRefV1
    ) -> tuple[ReviewAuthoritySourceBindingV4, ...]: ...


class ContextApplicationV3RpaResolver:
    """Resolve one exact current RPA V2 member from the admitted RPA aggregate."""

    def __init__(
        self,
        authority: object,
        resolver: RelationApplicationV2Resolver,
        *,
        currentness: object,
    ) -> None:
        try:
            admitted = admit_relation_application_authority_v2(
                authority,
                resolver,
                currentness=currentness,
            )
        except Exception as exc:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_RPA_NOT_CURRENT",
                "relation_application_authority_v2",
                cause_code=getattr(exc, "code", type(exc).__name__),
            ) from exc
        self._authority = authority
        self._rpa_resolver = resolver
        self._current_record_ids = {
            identity.as_text() for identity in admitted.currentness.current_record_ids
        }

    def expected_rpa_source_closure(
        self, record: RelationApplicationV2Record, reviewer_roster_ref: ReviewerRosterRefV1
    ) -> tuple[ReviewAuthoritySourceBindingV4, ...]:
        try:
            return self._rpa_resolver.expected_relation_application_v2_source_closure(
                record, reviewer_roster_ref
            )
        except Exception as exc:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_RPA_CLOSURE_MISMATCH",
                "relation_application_v2.source_closure",
                cause_code=getattr(exc, "code", type(exc).__name__),
            ) from exc

    def resolve_current_rpa_member(
        self,
        application_id: object,
        candidate_id: str,
        candidate_identity: DigestReferenceV1,
        source_instance_id: str,
    ) -> ResolvedRpaV2Member:
        if (
            getattr(application_id, "kind", None)
            is not AuthorityIdentityKind.RELATION_APPLICATION_V2
        ):
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_RPA_REQUIRED", "relation_application_v2_id"
            )
        records = [
            record
            for record in self._authority.relation_application_v2_records
            if record.application_id == application_id
            and record.record_id.as_text() in self._current_record_ids
        ]
        if len(records) != 1:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_RPA_NOT_CURRENT", "relation_application_v2_id"
            )
        record = records[0]
        matches = [
            member
            for member in record.members
            if member.candidate_id == candidate_id
            and member.candidate_identity_digest_reference == candidate_identity
            and member.source_instance_id == source_instance_id
        ]
        if len(matches) != 1:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_MEMBER_MISMATCH", "relation_application_v2.member"
            )
        return ResolvedRpaV2Member(record, matches[0])


class ContextApplicationV3Resolver:
    """Resolve V3 sources and reconstruct immutable V4 dependency closure."""

    def __init__(
        self,
        source_resolver: AuthoritySourceResolver,
        *,
        base_authority_binding: ContextAuthoritySourceBindingV2,
        rpa_member_resolver: ContextApplicationV3RpaMemberResolver,
    ) -> None:
        self._source_resolver = source_resolver
        self._base_binding = base_authority_binding
        self._rpa_member_resolver = rpa_member_resolver
        self._v2_resolver = ContextApplicationV2Resolver(
            source_resolver,
            base_authority_binding=base_authority_binding,
        )
        self._base_document: Mapping[str, object] | None = None
        self._validator: AuthorityValidator | None = None
        self._context_application_v3_records: tuple[ContextApplicationV3Record, ...] = ()

    def set_context_application_v3_records(
        self, records: tuple[ContextApplicationV3Record, ...]
    ) -> None:
        """Provide the immutable endpoint set used by CPSR closure reconstruction."""

        if any(not isinstance(record, ContextApplicationV3Record) for record in records):
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_INPUT_INVALID", "context_application_v3_records"
            )
        self._context_application_v3_records = tuple(records)

    def _validated_base(self) -> tuple[AuthorityValidator, Mapping[str, object]]:
        if self._validator is not None and self._base_document is not None:
            return self._validator, self._base_document
        artifact = self._v2_resolver.resolve_source_binding(self._base_binding)
        if not isinstance(artifact.json_value, Mapping):
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_MISMATCH", "base_authority_v1"
            )
        document = dict(artifact.json_value)
        validator = AuthorityValidator(self._source_resolver)
        validator.validate(document)
        self._validator = validator
        self._base_document = document
        return validator, document

    def resolve_current_context_theorem(self, theorem_record_id: object) -> Mapping[str, object]:
        validator, _ = self._validated_base()
        try:
            return validator.require_current_context_theorem(theorem_record_id)
        except Exception as exc:
            raise ContextApplicationV3ResolutionError(
                getattr(exc, "code", "CONTEXT_APPLICATION_V3_CURRENTNESS_FAILED"),
                "theorem_record_id",
            ) from exc

    def resolve_member_source_instance(self, member: ContextApplicationMemberV3) -> object:
        if not isinstance(member, ContextApplicationMemberV3):
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_INPUT_INVALID", "member"
            )
        projected = ContextApplicationMemberV2(
            candidate_id=member.candidate_id,
            candidate_identity_digest_reference=member.candidate_identity_digest_reference,
            source_instance_id=member.source_instance_id,
            candidate_universe_binding=member.candidate_universe_binding,
            context_binding_v1=member.reviewed_context_binding_v1,
            precondition_attestations_v1=member.precondition_attestations_v1,
            member_evidence_refs=member.member_evidence_refs,
            context_member_bridge_attestation_v2=member.context_member_bridge_attestation_v2,
        )
        return self._v2_resolver.resolve_member_source_instance(projected)

    def resolve_acceptance_event_leaf_v4(self, reference: object) -> ReviewAcceptanceEventLeafV4:
        return self._source_resolver.resolve_acceptance_event_leaf_v4(reference)

    def resolve_v4_source_binding(self, binding: ReviewAuthoritySourceBindingV4) -> object:
        return self._source_resolver.resolve_v4_source_binding(binding)

    def resolve_v4_acceptance_evidence(self, evidence: AcceptanceEvidenceRefV1) -> object:
        return self._source_resolver.resolve_v4_acceptance_evidence(evidence)

    @staticmethod
    def _project(binding: SourceBindingDigestV1) -> ReviewAuthoritySourceBindingV4:
        return V1DependencySourceBindingToV4(binding)

    def _v1_event_closure(
        self, reference: ReviewEventRefV1, seen: set[str] | None = None
    ) -> list[ReviewAuthoritySourceBindingV4]:
        seen = set() if seen is None else set(seen)
        if reference.event_id in seen:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH", "theorem.acceptance"
            )
        seen.add(reference.event_id)
        try:
            artifact = self._source_resolver.resolve_acceptance_event_leaf(reference)
            event = artifact.json_value
            if not isinstance(event, Mapping):
                raise ValueError("V1 acceptance event is not an object")
            result = [
                self._project(
                    SourceBindingDigestV1(
                        "acceptance_event_leaf",
                        reference.path,
                        "manafold.m2.5.c.review-acceptance-event.v1",
                        reference.raw_sha256,
                    )
                )
            ]
            raw_sources = event.get("source_binding_digests")
            if not isinstance(raw_sources, list):
                raise ValueError("V1 event source bindings are not an array")
            for raw in raw_sources:
                if not isinstance(raw, Mapping):
                    raise ValueError("V1 event source binding is not an object")
                source = SourceBindingDigestV1(
                    raw["artifact_role"],
                    raw["path"],
                    raw.get("schema_or_null"),
                    bytes.fromhex(raw["raw_sha256"]),
                )
                result.append(self._project(source))
                if source.artifact_role == "acceptance_event_leaf":
                    nested = self._source_resolver.resolve_source_binding(source).json_value
                    if not isinstance(nested, Mapping) or not isinstance(
                        nested.get("event_id"), str
                    ):
                        raise ValueError("nested V1 acceptance event is invalid")
                    result.extend(
                        self._v1_event_closure(
                            ReviewEventRefV1(source.path, source.raw_sha256, nested["event_id"]),
                            seen,
                        )
                    )
            return result
        except ContextApplicationV3ResolutionError:
            raise
        except (ResolutionError, TypeError, ValueError, KeyError) as exc:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH", "theorem.acceptance"
            ) from exc

    def _review_event_ref_v1(self, theorem: Mapping[str, object]) -> ReviewEventRefV1:
        acceptance = theorem.get("acceptance")
        if not isinstance(acceptance, Mapping) or not isinstance(
            acceptance.get("review_event_ref"), Mapping
        ):
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH", "theorem.acceptance"
            )
        ref = acceptance["review_event_ref"]
        locator = ref.get("locator")
        if not isinstance(locator, Mapping) or locator.get("kind") != "event_id":
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH", "theorem.acceptance"
            )
        return ReviewEventRefV1(ref["path"], bytes.fromhex(ref["raw_sha256"]), locator["value"])

    def expected_context_application_v3_source_closure(
        self,
        record: ContextApplicationV3Record,
        reviewer_roster_ref: ReviewerRosterRefV1,
    ) -> tuple[ReviewAuthoritySourceBindingV4, ...]:
        """Reconstruct the immutable V4 closure independently of event claims."""

        validator, base_document = self._validated_base()
        theorem = self.resolve_current_context_theorem(record.theorem_record_id)
        base, model, available, _ = self._v2_resolver._base_context(self._base_binding)
        direct: list[ContextAuthoritySourceBindingV2] = []
        rpa_event_sources: list[ReviewAuthoritySourceBindingV4] = []
        b2_roles: set[str] = set()
        b1 = False
        for member in record.members:
            projected = ContextApplicationMemberV2(
                member.candidate_id,
                member.candidate_identity_digest_reference,
                member.source_instance_id,
                member.candidate_universe_binding,
                member.reviewed_context_binding_v1,
                member.precondition_attestations_v1,
                member.member_evidence_refs,
                member.context_member_bridge_attestation_v2,
            )
            candidate_bindings, candidate_available = self._v2_resolver._candidate_provenance(
                projected
            )
            direct.extend(candidate_bindings)
            available = tuple(available) + tuple(candidate_available)
            evidence_bindings, evidence_b2, evidence_b1 = self._v2_resolver._collect_evidence(
                self._v2_resolver._member_evidence(projected)
            )
            direct.extend(evidence_bindings)
            b2_roles.update(evidence_b2)
            b1 = b1 or evidence_b1

            resolved_rpa = self._rpa_member_resolver.resolve_current_rpa_member(
                AuthorityIdentityV1(
                    AuthorityIdentityKind.RELATION_APPLICATION_V2,
                    member.relation_application_v2_id_bytes,
                ),
                member.candidate_id,
                member.candidate_identity_digest_reference,
                member.source_instance_id,
            )
            rpa_event = self.resolve_acceptance_event_leaf_v4(
                resolved_rpa.record.review_event_ref_v4
            )
            rpa_event_sources.append(
                ReviewAuthoritySourceBindingV4(
                    "acceptance_event_leaf_v4",
                    resolved_rpa.record.review_event_ref_v4.path,
                    "manafold.m2.5.c.review-acceptance-event.v4",
                    resolved_rpa.record.review_event_ref_v4.raw_sha256,
                )
            )
            rpa_event_sources.extend(
                self._rpa_member_resolver.expected_rpa_source_closure(
                    resolved_rpa.record, rpa_event.reviewer_roster_ref
                )
            )

        theorem_bindings, theorem_b2, theorem_b1 = self._v2_resolver._walk_v1_dependencies(theorem)
        direct.extend(theorem_bindings)
        b2_roles.update(theorem_b2)
        b1 = b1 or theorem_b1
        context_closure = reconstruct_event_source_closure(
            fixed_bindings=(model,),
            direct_bindings=tuple(direct),
            available_bindings=available,
            b2_evidence_roles=b2_roles,
            b1_citation=b1,
        )
        result: list[ReviewAuthoritySourceBindingV4] = [
            self._project(
                SourceBindingDigestV1("declared_model", model.path, model.schema, model.raw_sha256)
            ),
            ReviewAuthoritySourceBindingV4(
                "reviewer_roster_leaf",
                reviewer_roster_ref.path,
                reviewer_roster_ref.schema,
                reviewer_roster_ref.raw_sha256,
            ),
        ]
        result.extend(
            self._project(
                SourceBindingDigestV1(
                    binding.artifact_role,
                    binding.path,
                    binding.schema,
                    binding.raw_sha256,
                )
            )
            for binding in context_closure
            if binding.artifact_role != "base_authority_v1"
        )
        result.extend(self._v1_event_closure(self._review_event_ref_v1(theorem)))
        result.extend(rpa_event_sources)
        return tuple(
            sorted(
                {encode_canonical(item.to_cbor()): item for item in result}.values(),
                key=lambda item: encode_canonical(item.to_cbor()),
            )
        )

    def expected_context_application_v3_supersession_source_closure(
        self,
        record: object,
        reviewer_roster_ref: ReviewerRosterRefV1,
    ) -> tuple[ReviewAuthoritySourceBindingV4, ...]:
        """Reconstruct CPSR closure from its own evidence and immutable endpoints."""

        from mtgml.authority import ContextApplicationV3SupersessionRecord

        if not isinstance(record, ContextApplicationV3SupersessionRecord):
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH",
                "supersession_record",
            )
        endpoints: list[ContextApplicationV3Record] = []
        by_id = {item.record_id.as_text(): item for item in self._context_application_v3_records}
        for endpoint_id in (record.superseded_record_id, record.replacement_record_id):
            if endpoint_id is None:
                continue
            endpoint = by_id.get(endpoint_id.as_text())
            if endpoint is None:
                raise ContextApplicationV3ResolutionError(
                    "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH",
                    "supersession.endpoint",
                )
            endpoints.append(endpoint)

        try:
            evidence_bindings, _, _ = self._v2_resolver._collect_evidence(
                record.source_evidence_refs
            )
            result: list[ReviewAuthoritySourceBindingV4] = [
                self._project(
                    SourceBindingDigestV1(
                        binding.artifact_role,
                        binding.path,
                        binding.schema,
                        binding.raw_sha256,
                    )
                )
                for binding in evidence_bindings
            ]
            result.append(
                ReviewAuthoritySourceBindingV4(
                    "reviewer_roster_leaf",
                    reviewer_roster_ref.path,
                    reviewer_roster_ref.schema,
                    reviewer_roster_ref.raw_sha256,
                )
            )
            for endpoint in endpoints:
                event = self.resolve_acceptance_event_leaf_v4(endpoint.review_event_ref_v4)
                result.append(
                    ReviewAuthoritySourceBindingV4(
                        "acceptance_event_leaf_v4",
                        endpoint.review_event_ref_v4.path,
                        "manafold.m2.5.c.review-acceptance-event.v4",
                        endpoint.review_event_ref_v4.raw_sha256,
                    )
                )
                result.extend(
                    self.expected_context_application_v3_source_closure(
                        endpoint,
                        event.reviewer_roster_ref,
                    )
                )
            unique = {encode_canonical(item.to_cbor()): item for item in result}
            return tuple(sorted(unique.values(), key=lambda item: encode_canonical(item.to_cbor())))
        except ContextApplicationV3ResolutionError:
            raise
        except Exception as exc:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH",
                "supersession_record.source_closure",
                cause_code=getattr(exc, "code", type(exc).__name__),
            ) from exc


class ContextApplicationV3AuthorityResolver:
    """Container-level V3 provenance and shared-snapshot boundary."""

    def __init__(self, resolver: ContextApplicationV3Resolver | None = None) -> None:
        self._resolver = resolver

    @staticmethod
    def _same_binding(left: object, right: object) -> bool:
        return (
            getattr(left, "artifact_role", None) == getattr(right, "artifact_role", None)
            and getattr(left, "path", None) == getattr(right, "path", None)
            and getattr(left, "schema", getattr(left, "schema_or_null", None))
            == getattr(right, "schema", getattr(right, "schema_or_null", None))
            and getattr(left, "raw_sha256", None) == getattr(right, "raw_sha256", None)
        )

    @staticmethod
    def require_cross_version_eligibility(
        *,
        v2_member_facts: tuple[tuple[tuple[bytes, str], tuple[str, ...], tuple[str, ...]], ...],
        v3_member_facts: tuple[tuple[tuple[bytes, str], tuple[str, ...], tuple[str, ...]], ...],
    ) -> None:
        """Enforce the exact-role V2 / divergent-role V3 partition."""

        v2_keys = {fact[0] for fact in v2_member_facts}
        v3_keys = {fact[0] for fact in v3_member_facts}
        if len(v2_keys) != len(v2_member_facts) or len(v3_keys) != len(v3_member_facts):
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_AUTHORITY_VERSION_AMBIGUOUS", "current_members"
            )
        overlap = v2_keys & v3_keys
        if overlap:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_AUTHORITY_VERSION_AMBIGUOUS", "current_members"
            )
        for _, historical, reviewed in v2_member_facts:
            if historical != reviewed:
                raise ContextApplicationV3ResolutionError(
                    "CONTEXT_AUTHORITY_VERSION_ELIGIBILITY_MISMATCH", "context_v2.member"
                )
        for _, historical, reviewed in v3_member_facts:
            if historical == reviewed:
                raise ContextApplicationV3ResolutionError(
                    "CONTEXT_AUTHORITY_VERSION_ELIGIBILITY_MISMATCH", "context_v3.member"
                )

    @staticmethod
    def _source_roles(resolved: object) -> tuple[str, ...]:
        source_record = getattr(resolved, "source_instance_record", None)
        if not isinstance(source_record, Mapping):
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_MISMATCH", "source_instance.relation_binding"
            )
        relation_binding = source_record.get("relation_binding")
        if not isinstance(relation_binding, Mapping):
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_MISMATCH", "source_instance.relation_binding"
            )
        raw_participants = relation_binding.get("participant_bindings")
        if not isinstance(raw_participants, list):
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_MISMATCH",
                "source_instance.participant_bindings",
            )
        try:
            ordered = sorted(raw_participants, key=lambda item: item["position"])
            return tuple(item["role"] for item in ordered)
        except (KeyError, TypeError) as exc:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_MISMATCH",
                "source_instance.participant_bindings",
            ) from exc

    @staticmethod
    def _reviewed_roles(member: object) -> tuple[str, ...]:
        context_binding = getattr(member, "reviewed_context_binding_v1", None)
        if context_binding is None:
            context_binding = getattr(member, "context_binding_v1", None)
        if not isinstance(context_binding, list) or len(context_binding) != 4:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_INPUT_INVALID", "member.context_binding"
            )
        participants = context_binding[2]
        if not isinstance(participants, list):
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_INPUT_INVALID", "member.context_binding.participant_roles"
            )
        try:
            return tuple(item[1] for item in sorted(participants, key=lambda item: item[0]))
        except (IndexError, TypeError) as exc:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_INPUT_INVALID", "member.context_binding.participant_roles"
            ) from exc

    def _record_role_facts(
        self,
        records: tuple[object, ...],
        *,
        v3: bool,
    ) -> tuple[tuple[tuple[bytes, str], tuple[str, ...], tuple[str, ...]], ...]:
        if self._resolver is None:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_AUTHORITY_VERSION_ELIGIBILITY_MISMATCH", "resolver"
            )
        facts: list[tuple[tuple[bytes, str], tuple[str, ...], tuple[str, ...]]] = []
        for record in records:
            members = getattr(record, "members", None)
            if not isinstance(members, tuple):
                raise ContextApplicationV3ResolutionError(
                    "CONTEXT_AUTHORITY_VERSION_ELIGIBILITY_MISMATCH", "current_members"
                )
            for member in members:
                resolved = (
                    self._resolver.resolve_member_source_instance(member)
                    if v3
                    else self._resolver._v2_resolver.resolve_member_source_instance(member)
                )
                key = (
                    member.candidate_identity_digest_reference.digest_bytes,
                    member.source_instance_id,
                )
                facts.append((key, self._source_roles(resolved), self._reviewed_roles(member)))
        return tuple(facts)

    def evaluate_authority(
        self,
        authority: ContextApplicationAuthorityV3,
        *,
        relation_authority: object,
        v2_current_records: tuple[ContextApplicationV2Record, ...],
        host_read_model: object | None = None,
        candidate_records: Mapping[str, Mapping[str, object]] | None = None,
    ) -> object:
        """Run closure, secure admission/currentness, and version partition together."""

        if authority.application_host_bindings_v3 and host_read_model is None:
            if self._resolver is None:
                raise ContextApplicationV3ResolutionError(
                    "HOST_AUTHORITY_BINDING_REQUIRED", "host_binding_authority_v2_binding"
                )
            from context_application_v3_host_binding import admit_host_binding_authority_v2

            host_read_model = admit_host_binding_authority_v2(
                self._resolver._source_resolver,
                authority.host_binding_authority_v2_binding,
            )

        self.validate_source_closure(
            authority,
            relation_authority=relation_authority,
            host_read_model=host_read_model,
        )
        currentness = self.evaluate_currentness(authority)
        current_keys = {identity.as_text() for identity in currentness.current_record_ids}
        v3_current = tuple(
            record
            for record in authority.context_application_v3_records
            if record.record_id.as_text() in current_keys
        )
        self.require_cross_version_eligibility(
            v2_member_facts=self._record_role_facts(v2_current_records, v3=False),
            v3_member_facts=self._record_role_facts(v3_current, v3=True),
        )
        if authority.application_host_bindings_v3:
            if host_read_model is None or candidate_records is None:
                raise ContextApplicationV3ResolutionError(
                    "HOST_AUTHORITY_BINDING_REQUIRED", "application_host_bindings_v3"
                )
            from context_application_v3_host_binding import validate_application_host_binding_v3

            links_by_application = {
                link.application_semantic_id: link
                for link in authority.application_host_bindings_v3
            }
            for record in v3_current:
                link = links_by_application.get(record.application_id)
                if link is None:
                    raise ContextApplicationV3ResolutionError(
                        "APPLICATION_HOST_BINDING_INVALID", "application_host_bindings_v3"
                    )
                try:
                    validate_application_host_binding_v3(
                        record,
                        link,
                        host_read_model,
                        candidate_records,
                    )
                except Exception as exc:
                    raise ContextApplicationV3ResolutionError(
                        getattr(exc, "code", "HOST_AUTHORITY_INVALID"),
                        "application_host_bindings_v3",
                    ) from exc
        return currentness

    @classmethod
    def require_shared_snapshots(
        cls, context_authority: ContextApplicationAuthorityV3, relation_authority: object
    ) -> None:
        direct_pairs = (
            (
                context_authority.base_authority_v1_binding,
                getattr(relation_authority, "base_authority_v1_binding", None),
                "base_authority_v1_binding",
            ),
            (
                context_authority.candidate_universe_binding,
                getattr(relation_authority, "candidate_universe_binding", None),
                "candidate_universe_binding",
            ),
        )
        for context_binding, relation_binding, location in direct_pairs:
            if relation_binding is None or not cls._same_binding(context_binding, relation_binding):
                raise ContextApplicationV3ResolutionError(
                    "CONTEXT_APPLICATION_V3_SHARED_SNAPSHOT_MISMATCH", location
                )
        context_sources = getattr(context_authority, "source_bindings", None)
        relation_sources = getattr(relation_authority, "source_bindings", None)
        if context_sources is None or relation_sources is None:
            return
        shared_roles = {
            "declared_model",
            "rev3_candidate_census",
            "rev3_pair_aggregates",
            "rev3_card_requirement_map",
            "rev3_deck_row_source_resolution",
            "rev3_osi_source_records",
            "rev3_source_index",
            "b1_final_citations",
            "b1_final_closure",
            "b2_catalog",
            "b2_classifications",
            "b2_closure",
        }
        context_by_role = {
            item.artifact_role: item
            for item in context_sources
            if item.artifact_role in shared_roles
        }
        relation_by_role = {
            item.artifact_role: item
            for item in relation_sources
            if item.artifact_role in shared_roles
        }
        for role in sorted(shared_roles):
            context_binding = context_by_role.get(role)
            relation_binding = relation_by_role.get(role)
            if (context_binding is None) != (relation_binding is None) or (
                context_binding is not None
                and not cls._same_binding(context_binding, relation_binding)
            ):
                raise ContextApplicationV3ResolutionError(
                    "CONTEXT_APPLICATION_V3_SHARED_SNAPSHOT_MISMATCH", role
                )

    @staticmethod
    def require_exact_projection(
        authority: ContextApplicationAuthorityV3,
    ) -> None:
        by_role = {item.artifact_role: item for item in authority.source_bindings}
        for role, projection in (
            ("base_authority_v1", authority.base_authority_v1_binding),
            ("candidate_universe", authority.candidate_universe_binding),
            ("relation_authority_v2", authority.relation_application_authority_v2_binding),
        ):
            if by_role.get(role) != projection:
                raise ContextApplicationV3ResolutionError(
                    "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH", role
                )
        host = authority.host_binding_authority_v2_binding
        if host is None and "host_binding_authority_v2" in by_role:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH",
                "host_binding_authority_v2_binding",
            )
        if host is not None and by_role.get("host_binding_authority_v2") != host:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH",
                "host_binding_authority_v2_binding",
            )

    @staticmethod
    def _context_binding_from_v4(binding: ReviewAuthoritySourceBindingV4) -> object:
        from mtgml.authority import ContextAuthoritySourceBindingV3

        try:
            return ContextAuthoritySourceBindingV3(
                binding.artifact_role,
                binding.path,
                binding.schema,
                binding.raw_sha256,
            )
        except Exception as exc:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH",
                "source_bindings",
                cause_code=type(exc).__name__,
            ) from exc

    @classmethod
    def _require_exact_bindings(
        cls,
        actual: object,
        expected: object,
        location: str,
    ) -> None:
        actual_values = tuple(actual)
        expected_values = tuple(expected)
        actual_bytes = tuple(encode_canonical(item.to_cbor()) for item in actual_values)
        expected_bytes = tuple(encode_canonical(item.to_cbor()) for item in expected_values)
        if actual_bytes != expected_bytes or len(set(actual_bytes)) != len(actual_bytes):
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH", location
            )

    def validate_source_closure(
        self,
        authority: ContextApplicationAuthorityV3,
        *,
        relation_authority: object,
        host_read_model: object | None = None,
    ) -> None:
        """Reconstruct all three container-level source sets before evaluation."""

        self.validate_container_shape(authority)
        if self._resolver is None:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH", "resolver"
            )
        self.validate_event_closure_boundary(authority)
        self.require_shared_snapshots(authority, relation_authority)

        rpa_resolver = getattr(self._resolver._rpa_member_resolver, "_rpa_resolver", None)
        expected_relation = getattr(
            rpa_resolver, "validate_relation_application_authority_v2_source_closure", None
        )
        if not callable(expected_relation):
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH",
                "relation_source_bindings",
            )
        try:
            reconstructed_relation = expected_relation(relation_authority)
        except Exception as exc:
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH",
                "relation_source_bindings",
                cause_code=getattr(exc, "code", type(exc).__name__),
            ) from exc
        self._require_exact_bindings(
            authority.relation_source_bindings,
            reconstructed_relation,
            "relation_source_bindings",
        )

        expected_context = {
            encode_canonical(
                authority.base_authority_v1_binding.to_cbor()
            ): authority.base_authority_v1_binding,
            encode_canonical(
                authority.candidate_universe_binding.to_cbor()
            ): authority.candidate_universe_binding,
            encode_canonical(
                authority.relation_application_authority_v2_binding.to_cbor()
            ): authority.relation_application_authority_v2_binding,
        }
        if authority.application_host_bindings_v3:
            host_projection = authority.host_binding_authority_v2_binding
            if host_projection is None:
                raise ContextApplicationV3ResolutionError(
                    "HOST_AUTHORITY_BINDING_REQUIRED", "host_binding_authority_v2_binding"
                )
            expected_context[encode_canonical(host_projection.to_cbor())] = host_projection
        for record in authority.context_application_v3_records:
            event = self._resolver.resolve_acceptance_event_leaf_v4(record.review_event_ref_v4)
            closure = self._resolver.expected_context_application_v3_source_closure(
                record, event.reviewer_roster_ref
            )
            event_binding = ReviewAuthoritySourceBindingV4(
                "acceptance_event_leaf_v4",
                record.review_event_ref_v4.path,
                "manafold.m2.5.c.review-acceptance-event.v4",
                record.review_event_ref_v4.raw_sha256,
            )
            for binding in (event_binding, *closure):
                projected = self._context_binding_from_v4(binding)
                expected_context[encode_canonical(projected.to_cbor())] = projected
        for record in authority.context_application_v3_supersession_records:
            event = self._resolver.resolve_acceptance_event_leaf_v4(record.review_event_ref_v4)
            closure = self._resolver.expected_context_application_v3_supersession_source_closure(
                record, event.reviewer_roster_ref
            )
            event_binding = ReviewAuthoritySourceBindingV4(
                "acceptance_event_leaf_v4",
                record.review_event_ref_v4.path,
                "manafold.m2.5.c.review-acceptance-event.v4",
                record.review_event_ref_v4.raw_sha256,
            )
            for binding in (event_binding, *closure):
                projected = self._context_binding_from_v4(binding)
                expected_context[encode_canonical(projected.to_cbor())] = projected

        if authority.application_host_bindings_v3:
            if host_read_model is None:
                raise ContextApplicationV3ResolutionError(
                    "HOST_AUTHORITY_BINDING_REQUIRED", "host_binding_authority_v2_binding"
                )
            host_used = getattr(host_read_model, "used_source_bindings", None)
            if host_used is None:
                raise ContextApplicationV3ResolutionError(
                    "HOST_SOURCE_CLOSURE_MISMATCH", "host_binding_source_bindings"
                )
            context_by_role = {item.artifact_role: item for item in authority.source_bindings}
            for binding in host_used:
                if binding.artifact_role in {
                    "declared_model",
                    "candidate_universe",
                    "rev3_candidate_census",
                    "rev3_pair_aggregates",
                    "rev3_card_requirement_map",
                    "rev3_deck_row_source_resolution",
                    "rev3_osi_source_records",
                    "rev3_source_index",
                    "b2_catalog",
                    "b2_classifications",
                    "b2_closure",
                    "b1_final_citations",
                    "b1_final_closure",
                } and (
                    binding.artifact_role not in context_by_role
                    or not self._same_binding(context_by_role[binding.artifact_role], binding)
                ):
                    raise ContextApplicationV3ResolutionError(
                        "CONTEXT_APPLICATION_V3_SHARED_SNAPSHOT_MISMATCH",
                        binding.artifact_role,
                    )
            for binding in host_used:
                for role in ("declared_model", "candidate_universe"):
                    if binding.artifact_role == role:
                        context_binding = next(
                            (
                                item
                                for item in authority.source_bindings
                                if item.artifact_role == role
                            ),
                            None,
                        )
                        if context_binding is None or not self._same_binding(
                            context_binding, binding
                        ):
                            raise ContextApplicationV3ResolutionError(
                                "CONTEXT_APPLICATION_V3_SHARED_SNAPSHOT_MISMATCH", role
                            )

        expected_context_values = tuple(
            sorted(expected_context.values(), key=lambda item: encode_canonical(item.to_cbor()))
        )
        self._require_exact_bindings(
            authority.source_bindings,
            expected_context_values,
            "source_bindings",
        )
        if host_read_model is not None:
            expected_host = tuple(getattr(host_read_model, "used_source_bindings", ()))
            self._require_exact_bindings(
                authority.host_binding_source_bindings,
                expected_host,
                "host_binding_source_bindings",
            )

    @classmethod
    def validate_container_shape(cls, authority: ContextApplicationAuthorityV3) -> None:
        if not isinstance(authority, ContextApplicationAuthorityV3):
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_INPUT_INVALID", "authority"
            )
        cls.require_exact_projection(authority)
        if (
            authority.application_host_bindings_v3
            and authority.host_binding_authority_v2_binding is None
        ):
            raise ContextApplicationV3ResolutionError(
                "HOST_AUTHORITY_BINDING_REQUIRED", "host_binding_authority_v2_binding"
            )
        if (
            not authority.application_host_bindings_v3
            and authority.host_binding_authority_v2_binding is not None
        ):
            raise ContextApplicationV3ResolutionError(
                "HOST_AUTHORITY_BINDING_UNEXPECTED", "host_binding_authority_v2_binding"
            )
        if not authority.application_host_bindings_v3 and authority.host_binding_source_bindings:
            raise ContextApplicationV3ResolutionError(
                "HOST_AUTHORITY_BINDING_UNEXPECTED", "host_binding_source_bindings"
            )

    def evaluate_currentness(
        self,
        authority: ContextApplicationAuthorityV3,
        *,
        record_admitter: object | None = None,
        supersession_admitter: object | None = None,
    ) -> object:
        from context_application_v3_supersession import (
            ContextApplicationV3CurrentnessEvaluator,
            ContextApplicationV3SupersessionAdmissionValidator,
            admit_context_application_v3_supersession_record,
        )

        ContextApplicationV3AuthorityResolver.validate_container_shape(authority)
        if self._resolver is not None:
            self._resolver.set_context_application_v3_records(
                authority.context_application_v3_records
            )
            if record_admitter is None:
                from context_application_v3_review_admission import (
                    admit_context_application_v3_record,
                )

                def record_admitter(record: ContextApplicationV3Record) -> object:
                    return admit_context_application_v3_record(record, self._resolver)

            if supersession_admitter is None:

                def supersession_record_admitter(record: object) -> object:
                    return admit_context_application_v3_supersession_record(record, self._resolver)

                supersession_admitter = ContextApplicationV3SupersessionAdmissionValidator(
                    own_record_admitter=supersession_record_admitter
                )
        if record_admitter is None or not isinstance(
            supersession_admitter, ContextApplicationV3SupersessionAdmissionValidator
        ):
            raise ContextApplicationV3ResolutionError(
                "CONTEXT_APPLICATION_V3_CURRENTNESS_ADMISSION_REQUIRED",
                "currentness",
            )
        evaluator = ContextApplicationV3CurrentnessEvaluator(
            record_admitter=record_admitter,
            supersession_admitter=supersession_admitter,
        )
        return evaluator.evaluate(
            authority.context_application_v3_records,
            authority.context_application_v3_supersession_records,
        )

    def validate_event_closure_boundary(self, authority: ContextApplicationAuthorityV3) -> None:
        """Keep mutable Context/Host aggregates outside immutable V4 closures."""

        self.validate_container_shape(authority)
        forbidden = {
            "context_application_authority_v3",
            "relation_authority_v2",
            "host_binding_authority_v2",
            "host_binding_claim_record",
        }
        records = tuple(authority.context_application_v3_records) + tuple(
            authority.context_application_v3_supersession_records
        )
        for record in records:
            if self._resolver is None:
                raise ContextApplicationV3ResolutionError(
                    "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH", "resolver"
                )
            event = self._resolver.resolve_acceptance_event_leaf_v4(record.review_event_ref_v4)
            if any(binding.artifact_role in forbidden for binding in event.source_binding_digests):
                raise ContextApplicationV3ResolutionError(
                    "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH",
                    "review_event.source_binding_digests",
                )


__all__ = [
    "ContextApplicationV3AuthorityResolver",
    "ContextApplicationV3ResolutionError",
    "ContextApplicationV3Resolver",
    "ContextApplicationV3RpaResolver",
    "ResolvedRpaV2Member",
]
