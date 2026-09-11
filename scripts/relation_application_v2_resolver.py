"""Authoritative RPA V2 source/currentness composition over V1 infrastructure."""

from __future__ import annotations

import sys
from collections.abc import Iterable, Mapping
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from authority_source_resolver import (
    DECLARED_MODEL_SCHEMA,
    AuthoritySourceResolver,
    B2ArtifactBindingsV1,
    B2BoundaryReferenceV1,
    ResolvedArtifact,
)
from authority_validator import AuthorityValidator
from mtgml.authority import (
    ACCEPTANCE_EVENT_SCHEMA_V1,
    AUTHORITY_SCHEMA_V1,
    AcceptanceEvidenceRefV1,
    EvidenceRefV1,
    RecordKind,
    RelationApplicationV2Record,
    RelationApplicationV2SupersessionRecord,
    RelationAuthoritySourceBindingV2,
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

    @staticmethod
    def validate_b2_closure_v2_readiness(repo_root: Path, binding: object) -> object:
        """Validate the future C/RPA B2-v2 root without creating an RPA record."""

        from b2_closure_downstream_readiness import require_verified_b2_v2

        return require_verified_b2_v2(repo_root, binding)

    def require_current_relation_theorem(self, theorem_record_id: object) -> Mapping[str, object]:
        try:
            return self._authority_validator.require_current_relation_theorem(theorem_record_id)
        except Exception as exc:
            code = getattr(exc, "code", "RELATION_APPLICATION_V2_CURRENTNESS_FAILED")
            raise RelationApplicationV2ResolutionError(code, "theorem_record_id") from exc

    def resolve_relation_theorem_record(self, theorem_record_id: object) -> Mapping[str, object]:
        try:
            return self._authority_validator.require_validated_record(
                theorem_record_id,
                RecordKind.RELATION_THEOREM_RECORD,
                "RPA V2 theorem_record_id",
            )
        except Exception as exc:
            raise RelationApplicationV2ResolutionError(
                "RELATION_APPLICATION_V2_THEOREM_MISMATCH",
                "theorem_record_id",
                str(exc),
            ) from exc

    def validate_relation_member_proof_v1(
        self,
        member: Mapping[str, object],
        theorem: Mapping[str, object],
        label: str,
    ) -> None:
        """Reuse the validated V1 member-proof operation for RPA V2."""

        try:
            self._authority_validator.validate_relation_member_proof_v1(member, theorem, label)
        except Exception as exc:
            raise RelationApplicationV2ResolutionError(
                getattr(exc, "code", "RELATION_APPLICATION_V2_THEOREM_MISMATCH"),
                label,
                str(exc),
            ) from exc

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

    def resolve_relation_source_binding(self, binding: RelationAuthoritySourceBindingV2) -> object:
        if binding.artifact_role.startswith("rev3_"):
            return self._source_resolver.resolve_rev3_member(
                binding.path,
                binding.raw_sha256,
                binding.schema,
            )
        return self._source_resolver.resolve_repository_artifact(
            binding.path,
            binding.raw_sha256,
            binding.schema,
        )

    def _root_source_binding(self, role: str) -> SourceBindingDigestV1:
        raw_sources = self._authority_document.get("source_bindings")
        if not isinstance(raw_sources, list):
            raise RelationApplicationV2ResolutionError(
                "RPA_V2_SOURCE_CLOSURE_RECONSTRUCTION_FAILED", "base_authority.source_bindings"
            )
        matches = [
            raw
            for raw in raw_sources
            if isinstance(raw, Mapping) and raw.get("artifact_role") == role
        ]
        if len(matches) != 1:
            raise RelationApplicationV2ResolutionError(
                "RPA_V2_SOURCE_CLOSURE_RECONSTRUCTION_FAILED",
                f"base_authority.source_bindings[{role}]",
            )
        raw = matches[0]
        try:
            return SourceBindingDigestV1(
                role,
                raw["path"],
                raw.get("schema_or_null"),
                bytes.fromhex(raw["raw_sha256"]),
            )
        except (KeyError, TypeError, ValueError) as exc:
            raise RelationApplicationV2ResolutionError(
                "RPA_V2_SOURCE_CLOSURE_RECONSTRUCTION_FAILED",
                f"base_authority.source_bindings[{role}]",
            ) from exc

    def _declared_model_binding(self) -> SourceBindingDigestV1:
        model = self._authority_document.get("model_binding")
        if not isinstance(model, Mapping):
            raise RelationApplicationV2ResolutionError(
                "RPA_V2_SOURCE_CLOSURE_RECONSTRUCTION_FAILED", "base_authority.model_binding"
            )
        try:
            return SourceBindingDigestV1(
                "declared_model",
                model["path"],
                DECLARED_MODEL_SCHEMA,
                bytes.fromhex(model["raw_sha256"]),
            )
        except (KeyError, TypeError, ValueError) as exc:
            raise RelationApplicationV2ResolutionError(
                "RPA_V2_SOURCE_CLOSURE_RECONSTRUCTION_FAILED", "base_authority.model_binding"
            ) from exc

    @staticmethod
    def _nonempty_list(value: object) -> bool:
        return isinstance(value, list) and bool(value)

    @classmethod
    def _semantic_static_roles(cls, value: object) -> set[str]:
        """Find explicit B1/B2 semantic dependencies, never event claims."""

        roles: set[str] = set()

        def walk(node: object) -> None:
            if isinstance(node, Mapping):
                for key, child in node.items():
                    if key in {
                        "b2_boundary_refs",
                        "b2_family_refs",
                        "through_boundary_refs",
                        "positive_boundary_facts",
                    } and cls._nonempty_list(child):
                        roles.update({"b2_catalog", "b2_classifications", "b2_closure"})
                    if key in {
                        "b1_final_citation_refs",
                        "b1_citation_refs",
                        "b1_final_citations",
                    } and cls._nonempty_list(child):
                        roles.update({"b1_final_citations", "b1_final_closure"})
                    walk(child)
            elif isinstance(node, list):
                for child in node:
                    walk(child)

        walk(value)
        return roles

    @classmethod
    def _theorem_static_roles(cls, theorem: Mapping[str, object]) -> set[str]:
        roles = cls._semantic_static_roles(theorem)
        for precondition in theorem.get("preconditions", []):
            if not isinstance(precondition, Mapping):
                continue
            kind = precondition.get("precondition_kind")
            payload = precondition.get("payload")
            if kind == "b2_boundary":
                roles.update({"b2_catalog", "b2_classifications", "b2_closure"})
            elif kind == "class_projection" and isinstance(payload, list) and len(payload) == 9:
                if (isinstance(payload[6], list) and bool(payload[6])) or (
                    isinstance(payload[7], list) and bool(payload[7])
                ):
                    roles.update({"b2_catalog", "b2_classifications", "b2_closure"})
                if isinstance(payload[8], list) and payload[8]:
                    roles.update({"b1_final_citations", "b1_final_closure"})
        return roles

    def _b2_bindings(self) -> B2ArtifactBindingsV1:
        raw_sources = self._authority_document.get("source_bindings")
        if not isinstance(raw_sources, list):
            raise RelationApplicationV2ResolutionError(
                "B2_PRECONDITION_SOURCE_MISMATCH", "base_authority.source_bindings"
            )
        found: dict[str, SourceBindingDigestV1] = {}
        for raw in raw_sources:
            if not isinstance(raw, Mapping):
                continue
            role = raw.get("artifact_role")
            if role not in {"b2_catalog", "b2_classifications", "b2_closure"}:
                continue
            try:
                found[role] = SourceBindingDigestV1(
                    role,
                    raw["path"],
                    raw.get("schema_or_null"),
                    bytes.fromhex(raw["raw_sha256"]),
                )
            except (KeyError, TypeError, ValueError) as exc:
                raise RelationApplicationV2ResolutionError(
                    "B2_PRECONDITION_SOURCE_MISMATCH", role
                ) from exc
        if set(found) != {"b2_catalog", "b2_classifications", "b2_closure"}:
            raise RelationApplicationV2ResolutionError(
                "B2_PRECONDITION_SOURCE_MISMATCH", "base_authority.source_bindings"
            )
        return B2ArtifactBindingsV1(
            found["b2_catalog"],
            found["b2_classifications"],
            found["b2_closure"],
        )

    def resolve_relation_application_v2_b2_boundary(self, payload: object) -> None:
        if not isinstance(payload, list) or len(payload) != 4:
            raise RelationApplicationV2ResolutionError(
                "B2_PRECONDITION_SOURCE_MISMATCH", "b2_boundary"
            )
        family_id, lifecycle, _assignment_role, definition = payload
        if not all(
            isinstance(value, str) and value for value in (family_id, lifecycle, definition)
        ):
            raise RelationApplicationV2ResolutionError(
                "B2_PRECONDITION_SOURCE_MISMATCH", "b2_boundary"
            )
        bindings = self._b2_bindings()
        try:
            family = self._source_resolver.resolve_b2_requirement_family(family_id, bindings)
            if str(family.record.get("status", "")).lower() != str(lifecycle).lower():
                raise RelationApplicationV2ResolutionError(
                    "B2_PRECONDITION_LIFECYCLE_MISMATCH", "b2_boundary.lifecycle"
                )
            self._source_resolver.resolve_b2_boundary(
                family,
                B2BoundaryReferenceV1(
                    family_id=family_id,
                    precise_semantic_definition=definition,
                ),
            )
        except RelationApplicationV2ResolutionError:
            raise
        except Exception as exc:
            raise RelationApplicationV2ResolutionError(
                "B2_PRECONDITION_SOURCE_MISMATCH", "b2_boundary", str(exc)
            ) from exc

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
            if isinstance(node, Mapping) and {
                "authority_kind",
                "path",
                "locator",
                "raw_sha256",
            }.issubset(node):
                locator = node["locator"]
                if isinstance(locator, Mapping) and isinstance(locator.get("kind"), str):
                    try:
                        found.append(
                            EvidenceRefV1(
                                authority_kind=node["authority_kind"],
                                path=node["path"],
                                locator=(locator["kind"], locator.get("value")),
                                raw_sha256=bytes.fromhex(node["raw_sha256"]),
                            )
                        )
                        return
                    except (TypeError, ValueError):
                        pass
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
        self, reference: ReviewEventRefV1, seen: set[str] | None = None
    ) -> list[ReviewAuthoritySourceBindingV4]:
        seen = set() if seen is None else set(seen)
        if reference.event_id in seen:
            raise RelationApplicationV2ResolutionError(
                "RPA_V2_SOURCE_CLOSURE_RECONSTRUCTION_FAILED", "theorem.acceptance"
            )
        seen.add(reference.event_id)
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
                if source.artifact_role == "acceptance_event_leaf":
                    nested_artifact = self._source_resolver.resolve_source_binding(source)
                    nested_event = nested_artifact.json_value
                    if not isinstance(nested_event, Mapping):
                        raise ValueError("nested V1 acceptance event is not an object")
                    nested_id = nested_event.get("event_id")
                    if not isinstance(nested_id, str):
                        raise ValueError("nested V1 acceptance event has no event ID")
                    result.extend(
                        self._v1_event_closure(
                            ReviewEventRefV1(
                                source.path,
                                source.raw_sha256,
                                nested_id,
                            ),
                            seen,
                        )
                    )
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
        theorem = self.resolve_relation_theorem_record(record.theorem_record_id)
        direct: list[ReviewAuthoritySourceBindingV4] = [
            V1DependencySourceBindingToV4(self._declared_model_binding()),
            ReviewAuthoritySourceBindingV4(
                "reviewer_roster_leaf",
                reviewer_roster_ref.path,
                reviewer_roster_ref.schema,
                reviewer_roster_ref.raw_sha256,
            ),
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
            direct.extend(
                V1DependencySourceBindingToV4(self._root_source_binding(role))
                for role in self._semantic_static_roles(member.to_wire())
            )
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
        direct.extend(
            V1DependencySourceBindingToV4(self._root_source_binding(role))
            for role in self._theorem_static_roles(theorem)
        )
        unique = {encode_canonical(binding.to_cbor()): binding for binding in direct}
        result = tuple(sorted(unique.values(), key=lambda item: encode_canonical(item.to_cbor())))
        for binding in result:
            self._source_resolver.resolve_v4_source_binding(binding)
        return result

    def expected_relation_application_v2_supersession_source_closure(
        self,
        record: RelationApplicationV2SupersessionRecord,
        reviewer_roster_ref: ReviewerRosterRefV1,
        superseded_record: RelationApplicationV2Record,
        replacement_record: RelationApplicationV2Record | None,
    ) -> tuple[ReviewAuthoritySourceBindingV4, ...]:
        direct = self._project_evidence(record.source_evidence_refs)
        endpoints = (superseded_record,) + (
            () if replacement_record is None else (replacement_record,)
        )
        for endpoint in endpoints:
            event = self.resolve_acceptance_event_leaf_v4(endpoint.review_event_ref_v4)
            direct.append(
                ReviewAuthoritySourceBindingV4(
                    "acceptance_event_leaf_v4",
                    endpoint.review_event_ref_v4.path,
                    "manafold.m2.5.c.review-acceptance-event.v4",
                    endpoint.review_event_ref_v4.raw_sha256,
                )
            )
            direct.extend(
                self.expected_relation_application_v2_source_closure(
                    endpoint,
                    event.reviewer_roster_ref,
                )
            )
        unique = {encode_canonical(binding.to_cbor()): binding for binding in direct}
        result = tuple(sorted(unique.values(), key=lambda item: encode_canonical(item.to_cbor())))
        for binding in result:
            self._source_resolver.resolve_v4_source_binding(binding)
        return result

    @staticmethod
    def _relation_binding_from_v4(
        binding: ReviewAuthoritySourceBindingV4,
    ) -> RelationAuthoritySourceBindingV2:
        return RelationAuthoritySourceBindingV2(
            binding.artifact_role,
            binding.path,
            binding.schema,
            binding.raw_sha256,
        )

    def expected_relation_application_authority_v2_source_closure(
        self,
        authority: object,
    ) -> tuple[RelationAuthoritySourceBindingV2, ...]:
        source_bindings = tuple(authority.source_bindings)
        base = authority.base_authority_v1_binding
        candidate = authority.candidate_universe_binding
        by_key = {(binding.artifact_role, binding.path): binding for binding in source_bindings}
        if by_key.get(("base_authority_v1", base.path)) != base:
            raise RelationApplicationV2ResolutionError(
                "CONTAINER_SOURCE_CLOSURE_MISMATCH", "base_authority_v1_binding"
            )
        if by_key.get(("candidate_universe", candidate.path)) != candidate:
            raise RelationApplicationV2ResolutionError(
                "CONTAINER_SOURCE_CLOSURE_MISMATCH", "candidate_universe_binding"
            )
        direct: list[RelationAuthoritySourceBindingV2] = [base, candidate]
        records = tuple(authority.relation_application_v2_records)
        supersessions = tuple(authority.relation_application_v2_supersession_records)
        record_map = {record.record_id.as_text(): record for record in records}
        for record in records:
            event = self.resolve_acceptance_event_leaf_v4(record.review_event_ref_v4)
            event_closure = self.expected_relation_application_v2_source_closure(
                record,
                event.reviewer_roster_ref,
            )
            direct.append(
                RelationAuthoritySourceBindingV2(
                    "acceptance_event_leaf_v4",
                    record.review_event_ref_v4.path,
                    "manafold.m2.5.c.review-acceptance-event.v4",
                    record.review_event_ref_v4.raw_sha256,
                )
            )
            direct.extend(self._relation_binding_from_v4(item) for item in event_closure)
        for record in supersessions:
            event = self.resolve_acceptance_event_leaf_v4(record.review_event_ref_v4)
            superseded = record_map.get(record.superseded_record_id.as_text())
            replacement = (
                None
                if record.replacement_record_id is None
                else record_map.get(record.replacement_record_id.as_text())
            )
            if superseded is None or (
                record.replacement_record_id is not None and replacement is None
            ):
                raise RelationApplicationV2ResolutionError(
                    "CONTAINER_SOURCE_CLOSURE_MISMATCH", "supersession endpoint"
                )
            event_closure = self.expected_relation_application_v2_supersession_source_closure(
                record,
                event.reviewer_roster_ref,
                superseded,
                replacement,
            )
            direct.append(
                RelationAuthoritySourceBindingV2(
                    "acceptance_event_leaf_v4",
                    record.review_event_ref_v4.path,
                    "manafold.m2.5.c.review-acceptance-event.v4",
                    record.review_event_ref_v4.raw_sha256,
                )
            )
            direct.extend(self._relation_binding_from_v4(item) for item in event_closure)
        unique = {encode_canonical(binding.to_cbor()): binding for binding in direct}
        result = tuple(sorted(unique.values(), key=lambda item: encode_canonical(item.to_cbor())))
        for binding in result:
            self.resolve_relation_source_binding(binding)
        return result

    def validate_relation_application_authority_v2_source_closure(
        self,
        authority: object,
    ) -> tuple[RelationAuthoritySourceBindingV2, ...]:
        expected = self.expected_relation_application_authority_v2_source_closure(authority)
        actual = tuple(authority.source_bindings)
        actual_encoded = tuple(encode_canonical(item.to_cbor()) for item in actual)
        expected_encoded = tuple(encode_canonical(item.to_cbor()) for item in expected)
        if actual_encoded != expected_encoded or len(set(actual_encoded)) != len(actual_encoded):
            raise RelationApplicationV2ResolutionError(
                "CONTAINER_SOURCE_CLOSURE_MISMATCH", "source_bindings"
            )
        return expected


__all__ = [
    "RelationApplicationV2ResolutionError",
    "RelationApplicationV2Resolver",
]
