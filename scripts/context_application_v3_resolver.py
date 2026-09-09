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
    ContextApplicationMemberV2,
    ContextApplicationMemberV3,
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
        self._current_record_ids = {
            identity.as_text() for identity in admitted.currentness.current_record_ids
        }

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

    def resolve_acceptance_event_leaf_v4(
        self, reference: object
    ) -> ReviewAcceptanceEventLeafV4:
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
            del rpa_event
            rpa_event_sources.append(
                ReviewAuthoritySourceBindingV4(
                    "acceptance_event_leaf_v4",
                    resolved_rpa.record.review_event_ref_v4.path,
                    "manafold.m2.5.c.review-acceptance-event.v4",
                    resolved_rpa.record.review_event_ref_v4.raw_sha256,
                )
            )
            rpa_closure = getattr(self._rpa_member_resolver, "expected_rpa_source_closure", None)
            if callable(rpa_closure):
                rpa_event_sources.extend(rpa_closure(resolved_rpa.record, reviewer_roster_ref))

        theorem_bindings, theorem_b2, theorem_b1 = self._v2_resolver._walk_v1_dependencies(
            theorem
        )
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
                SourceBindingDigestV1(
                    "declared_model", model.path, model.schema, model.raw_sha256
                )
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


__all__ = [
    "ContextApplicationV3ResolutionError",
    "ContextApplicationV3Resolver",
    "ContextApplicationV3RpaResolver",
    "ResolvedRpaV2Member",
]
