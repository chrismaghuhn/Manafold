"""Authoritative RPA V2 source/currentness composition over V1 infrastructure."""

from __future__ import annotations

import sys
from collections.abc import Iterable, Mapping
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from authority_source_resolver import AuthoritySourceResolver, ResolvedArtifact
from authority_validator import AuthorityValidator
from mtgml.authority import (
    ACCEPTANCE_EVENT_SCHEMA_V1,
    AUTHORITY_SCHEMA_V1,
    AcceptanceEvidenceRefV1,
    EvidenceRefV1,
    RelationApplicationV2Record,
    ReviewAcceptanceEventLeafV4,
    ReviewAuthoritySourceBindingV4,
    ReviewerRosterRefV1,
    ReviewEventRefV1,
    SourceBindingDigestV1,
    V1DependencySourceBindingToV4,
)
from mtgml.persistence import encode_canonical


class RelationApplicationV2ResolutionError(ValueError):
    def __init__(self, code: str, location: str, message: str | None = None) -> None:
        self.code = code
        self.location = location
        super().__init__(message or f"{code} at {location}")


_STATIC_ROLE_BY_PATH: dict[str, tuple[str, str | None]] = {
    "sources/m2_5/closures/C/declared_interaction_model.v2.json": (
        "declared_model",
        "manafold.m2.5.c.declared-interaction-model.v2",
    ),
    "sources/m2_5/closures/C/interaction_candidate_universe.v2.json": (
        "candidate_universe",
        "manafold.m2.5.c.interaction-candidate-universe.v2",
    ),
    "sources/m2_5/closures/B2/requirement_family_catalog.v1.json": (
        "b2_catalog",
        "manafold.m2.5.b2.requirement-family-catalog.v1",
    ),
    "sources/m2_5/closures/B2/card_semantic_classifications.v1.json": (
        "b2_classifications",
        "manafold.m2.5.b2.card-semantic-classifications.v1",
    ),
    "sources/m2_5/closures/B2/classification_closure.v1.json": (
        "b2_closure",
        "manafold.m2.5.b2.classification-closure.v1",
    ),
    "sources/m2_5/closures/B1/official_authority_citations.v3.json": (
        "b1_final_citations",
        "manafold.m2.5.b1.official-authority-citations.v3",
    ),
    "sources/m2_5/closures/B1/official_authority_citation_closure.v2.json": (
        "b1_final_closure",
        "manafold.m2.5.b1.official-authority-citation-closure.v2",
    ),
}


class RelationApplicationV2Resolver:
    """Resolve RPA V2 dependencies without trusting the V4 event claim set."""

    def __init__(
        self,
        source_resolver: AuthoritySourceResolver,
        authority_validator: AuthorityValidator,
        authority_document: Mapping[str, object],
    ) -> None:
        self._source_resolver = source_resolver
        self._authority_validator = authority_validator
        self._authority_document = authority_document
        if authority_document.get("schema") != AUTHORITY_SCHEMA_V1:
            raise RelationApplicationV2ResolutionError(
                "AUTHORITY_SCHEMA_MISMATCH", "base_authority.schema"
            )

    def require_current_relation_theorem(self, theorem_record_id: object) -> Mapping[str, object]:
        try:
            return self._authority_validator.require_current_relation_theorem(theorem_record_id)
        except Exception as exc:
            code = getattr(exc, "code", "RELATION_APPLICATION_V2_CURRENTNESS_FAILED")
            raise RelationApplicationV2ResolutionError(code, "theorem_record_id") from exc

    def resolve_candidate_source_instance(self, *args: object) -> object:
        return self._source_resolver.resolve_candidate_source_instance(*args)

    def resolve_acceptance_event_leaf_v4(self, reference: object) -> ReviewAcceptanceEventLeafV4:
        return self._source_resolver.resolve_acceptance_event_leaf_v4(reference)

    def resolve_v4_source_binding(
        self, binding: ReviewAuthoritySourceBindingV4
    ) -> ResolvedArtifact:
        return self._source_resolver.resolve_v4_source_binding(binding)

    def resolve_v4_acceptance_evidence(self, evidence: AcceptanceEvidenceRefV1) -> object:
        return self._source_resolver.resolve_v4_acceptance_evidence(evidence)

    @staticmethod
    def _evidence_from_cbor(value: object) -> EvidenceRefV1 | None:
        if not isinstance(value, list) or len(value) != 4:
            return None
        if not isinstance(value[0], str) or not isinstance(value[1], str):
            return None
        if not isinstance(value[2], list) or len(value[2]) != 2:
            return None
        if not isinstance(value[3], bytes) or len(value[3]) != 32:
            return None
        try:
            return EvidenceRefV1(
                authority_kind=value[0],
                path=value[1],
                locator=(value[2][0], value[2][1]),
                raw_sha256=value[3],
            )
        except (TypeError, ValueError):
            return None

    @classmethod
    def _walk_evidence(cls, value: object) -> tuple[EvidenceRefV1, ...]:
        found: list[EvidenceRefV1] = []

        def walk(node: object) -> None:
            evidence = cls._evidence_from_cbor(node)
            if evidence is not None:
                found.append(evidence)
                return
            if isinstance(node, list):
                for child in node:
                    walk(child)
            elif isinstance(node, Mapping):
                for child in node.values():
                    walk(child)

        walk(value)
        return tuple(found)

    @staticmethod
    def _source_binding_for_evidence(reference: EvidenceRefV1) -> SourceBindingDigestV1:
        if reference.authority_kind == "model":
            role, schema = "declared_model", "manafold.m2.5.c.declared-interaction-model.v2"
        elif reference.authority_kind == "c_candidate":
            role, schema = "candidate_universe", "manafold.m2.5.c.interaction-candidate-universe.v2"
        elif reference.authority_kind == "rev3":
            role, schema = "rev3_source", None
        elif reference.authority_kind == "b2" or reference.authority_kind == "b1_final":
            role, schema = _STATIC_ROLE_BY_PATH.get(reference.path, ("", None))
        elif reference.authority_kind == "reviewer_roster":
            role, schema = "reviewer_roster_leaf", "manafold.m2.5.c.reviewer-roster.v1"
        elif reference.authority_kind == "acceptance_event":
            role, schema = "acceptance_event_leaf", ACCEPTANCE_EVENT_SCHEMA_V1
        else:
            role, schema = "", None
        if not role:
            raise RelationApplicationV2ResolutionError("SOURCE_BINDING_UNMAPPABLE", reference.path)
        return SourceBindingDigestV1(role, reference.path, schema, reference.raw_sha256)

    def _project_evidence(
        self, references: Iterable[EvidenceRefV1]
    ) -> list[ReviewAuthoritySourceBindingV4]:
        projected: list[ReviewAuthoritySourceBindingV4] = []
        for reference in references:
            try:
                projected.append(
                    V1DependencySourceBindingToV4(self._source_binding_for_evidence(reference))
                )
            except (TypeError, ValueError) as exc:
                raise RelationApplicationV2ResolutionError(
                    "SOURCE_BINDING_UNMAPPABLE", reference.path
                ) from exc
        return projected

    def _v1_event_closure(
        self, reference: ReviewEventRefV1
    ) -> list[ReviewAuthoritySourceBindingV4]:
        try:
            artifact = self._source_resolver.resolve_acceptance_event_leaf(reference)
            event = artifact.json_value
            if not isinstance(event, Mapping):
                raise ValueError("V1 acceptance event is not an object")
            result = [
                V1DependencySourceBindingToV4(
                    SourceBindingDigestV1(
                        "acceptance_event_leaf",
                        reference.path,
                        ACCEPTANCE_EVENT_SCHEMA_V1,
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
                result.append(V1DependencySourceBindingToV4(source))
            return result
        except RelationApplicationV2ResolutionError:
            raise
        except (KeyError, TypeError, ValueError) as exc:
            raise RelationApplicationV2ResolutionError(
                "RPA_V2_SOURCE_CLOSURE_RECONSTRUCTION_FAILED", "theorem.acceptance"
            ) from exc

    def expected_relation_application_v2_source_closure(
        self,
        record: RelationApplicationV2Record,
        reviewer_roster_ref: object,
    ) -> tuple[ReviewAuthoritySourceBindingV4, ...]:
        if not isinstance(reviewer_roster_ref, ReviewerRosterRefV1):
            raise RelationApplicationV2ResolutionError(
                "RPA_V2_SOURCE_CLOSURE_RECONSTRUCTION_FAILED", "reviewer_roster_ref"
            )
        theorem = self.require_current_relation_theorem(record.theorem_record_id)
        direct: list[ReviewAuthoritySourceBindingV4] = [
            ReviewAuthoritySourceBindingV4(
                "reviewer_roster_leaf",
                reviewer_roster_ref.path,
                reviewer_roster_ref.schema,
                reviewer_roster_ref.raw_sha256,
            )
        ]
        for member in record.members:
            binding = member.candidate_universe_binding
            source_instance = self._source_resolver.resolve_candidate_source_instance(
                member.candidate_id,
                member.candidate_identity_digest_reference.to_wire(),
                member.source_instance_id,
                SourceBindingDigestV1("candidate_universe", binding[0], binding[1], binding[2]),
            )
            direct.append(
                ReviewAuthoritySourceBindingV4(
                    "candidate_universe", binding[0], binding[1], binding[2]
                )
            )
            direct.append(
                V1DependencySourceBindingToV4(
                    SourceBindingDigestV1(
                        "rev3_source",
                        source_instance.source_artifact.path,
                        None,
                        bytes.fromhex(source_instance.source_artifact.raw_sha256),
                    )
                )
            )
            direct.extend(self._project_evidence(self._walk_evidence(member.to_cbor())))
        acceptance = theorem.get("acceptance")
        if not isinstance(acceptance, Mapping) or not isinstance(
            acceptance.get("review_event_ref"), Mapping
        ):
            raise RelationApplicationV2ResolutionError(
                "RPA_V2_SOURCE_CLOSURE_RECONSTRUCTION_FAILED", "theorem.acceptance"
            )
        event_ref = acceptance["review_event_ref"]
        locator = event_ref.get("locator")
        if not isinstance(locator, Mapping) or locator.get("kind") != "event_id":
            raise RelationApplicationV2ResolutionError(
                "RPA_V2_SOURCE_CLOSURE_RECONSTRUCTION_FAILED", "theorem.acceptance.review_event_ref"
            )
        v1_ref = ReviewEventRefV1(
            path=event_ref["path"],
            raw_sha256=bytes.fromhex(event_ref["raw_sha256"]),
            event_id=locator["value"],
        )
        direct.extend(self._v1_event_closure(v1_ref))
        direct.extend(self._project_evidence(self._walk_evidence(theorem)))
        unique = {encode_canonical(binding.to_cbor()): binding for binding in direct}
        result = tuple(sorted(unique.values(), key=lambda item: encode_canonical(item.to_cbor())))
        for binding in result:
            self._source_resolver.resolve_v4_source_binding(binding)
        return result


__all__ = [
    "RelationApplicationV2ResolutionError",
    "RelationApplicationV2Resolver",
]
