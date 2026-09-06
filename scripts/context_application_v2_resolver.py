"""Rules-neutral V2 source resolution and exact source-closure algebra.

This module owns byte/source binding resolution for ContextApplicationV2 Slice 2.
It deliberately does not decide whether reviewed values, theorem statements, or
human acceptance are semantically correct.
"""

from __future__ import annotations

import io
import json
import re
import zipfile
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from typing import Final, cast

from authority_source_resolver import (
    AuthoritySourceResolver,
    Locator,
    ResolvedArtifact,
    ResolvedSourceInstance,
)
from authority_validator import AuthorityValidator
from mtgml.authority import (
    ACCEPTANCE_EVENT_SCHEMA_V3,
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKindV3,
    ContextApplicationMemberV2,
    ContextApplicationV2Record,
    ContextApplicationV2SupersessionRecord,
    ContextAuthoritySourceBindingV2,
    ContextAuthoritySourceRegistryEntryV2,
    DigestReferenceV1,
    EvidenceRefV1,
    ReviewAcceptanceEventInputV3,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewEventRefV3,
    ReviewMode,
    SourceBindingDigestV1,
    context_authority_source_registry_v2,
)
from mtgml.persistence import encode_canonical


V3_RESOLUTION_CODES: Final = frozenset(
    {
        "V3_EVENT_REFERENCE_INVALID",
        "V3_EVENT_SCHEMA_INVALID",
        "V3_EVENT_DECISION_INVALID",
        "CHECKLIST_V2_MISMATCH",
        "REVIEW_MODE_INVALID",
        "REVIEW_EVIDENCE_MISSING",
        "REVIEW_EVIDENCE_INVALID",
        "V3_EVENT_IDENTITY_INVALID",
        "V3_EVENT_SOURCE_INVALID",
    }
)


class ContextApplicationV2ResolutionError(ValueError):
    """Raised when a V2 source or exact closure is not resolvable."""

    def __init__(self, message: str, *, code: str | None = None) -> None:
        self.code = code
        super().__init__(message)


@dataclass(frozen=True)
class ResolvedContextEvidence:
    binding: ContextAuthoritySourceBindingV2 | None
    artifact: ResolvedArtifact
    locator: Locator
    value: object


@dataclass(frozen=True)
class ResolvedReviewAcceptanceEventV3:
    reference: ReviewEventRefV3
    artifact: ResolvedArtifact
    event_id: str
    event: ReviewAcceptanceEventInputV3


def _fail(
    message: str,
    *,
    code: str | None = None,
) -> ContextApplicationV2ResolutionError:
    if code is not None and code not in V3_RESOLUTION_CODES:
        raise ValueError(f"unknown V3 resolver diagnostic code: {code}")
    return ContextApplicationV2ResolutionError(message, code=code)


def _binding_key(binding: ContextAuthoritySourceBindingV2) -> tuple[str, str]:
    return binding.artifact_role, binding.path


def _binding_bytes(binding: ContextAuthoritySourceBindingV2) -> bytes:
    return encode_canonical(binding.to_cbor())


def _registry() -> dict[str, ContextAuthoritySourceRegistryEntryV2]:
    return dict(context_authority_source_registry_v2())


def _registry_entry(
    role: str, path: str, schema: str | None
) -> ContextAuthoritySourceRegistryEntryV2:
    entry = _registry().get(role)
    if entry is None:
        raise _fail(f"unknown V2 source role: {role!r}")
    if re.fullmatch(entry.path_pattern, path) is None or schema != entry.schema:
        raise _fail(f"V2 source role/path/schema mismatch for {role!r}")
    return entry


def _validate_binding(
    binding: ContextAuthoritySourceBindingV2,
) -> ContextAuthoritySourceRegistryEntryV2:
    if not isinstance(binding, ContextAuthoritySourceBindingV2):
        raise _fail("V2 source binding has the wrong type")
    entry = _registry_entry(binding.artifact_role, binding.path, binding.schema)
    if binding.artifact_role == "reviewer_roster_leaf":
        digest_component = binding.path.rsplit("/", 1)[-1].removesuffix(".json")
        if digest_component != binding.raw_sha256.hex():
            raise _fail("reviewer roster leaf path is not bound to its raw digest")
    return entry


def canonical_source_bindings(
    bindings: Iterable[ContextAuthoritySourceBindingV2],
    *,
    reject_duplicates: bool = False,
) -> tuple[ContextAuthoritySourceBindingV2, ...]:
    """Return a duplicate-free sequence sorted by complete canonical CBOR bytes."""

    by_key: dict[tuple[str, str], ContextAuthoritySourceBindingV2] = {}
    for binding in bindings:
        _validate_binding(binding)
        key = _binding_key(binding)
        previous = by_key.get(key)
        if previous is not None:
            if previous != binding:
                raise _fail(f"conflicting V2 source binding for role/path {key!r}")
            if reject_duplicates:
                raise _fail(f"duplicate V2 source binding for role/path {key!r}")
            continue
        by_key[key] = binding
    return tuple(sorted(by_key.values(), key=_binding_bytes))


def _reject_event_cycle(bindings: Iterable[ContextAuthoritySourceBindingV2]) -> None:
    for binding in bindings:
        if binding.artifact_role in {
            "acceptance_event_leaf_v3",
            "context_application_authority_v2",
        }:
            raise _fail(f"event closure contains forbidden role {binding.artifact_role!r}")


def _require_role(
    bindings: Mapping[str, ContextAuthoritySourceBindingV2], role: str
) -> ContextAuthoritySourceBindingV2:
    binding = bindings.get(role)
    if binding is None:
        raise _fail(f"required V2 source binding is missing: {role}")
    return binding


def reconstruct_event_source_closure(
    *,
    fixed_bindings: Sequence[ContextAuthoritySourceBindingV2],
    direct_bindings: Sequence[ContextAuthoritySourceBindingV2],
    available_bindings: Sequence[ContextAuthoritySourceBindingV2] = (),
    b2_evidence_roles: Iterable[str] = (),
    b1_citation: bool = False,
    host_bindings: Sequence[ContextAuthoritySourceBindingV2] = (),
) -> tuple[ContextAuthoritySourceBindingV2, ...]:
    """Reconstruct one event's exact structural source set."""

    seed_bindings = tuple(fixed_bindings) + tuple(direct_bindings) + tuple(host_bindings)
    all_bindings = seed_bindings + tuple(available_bindings)
    _reject_event_cycle(seed_bindings)
    unique = canonical_source_bindings(all_bindings)
    by_role = {binding.artifact_role: binding for binding in unique}

    dependency_roles = set(b2_evidence_roles)
    invalid_b2 = dependency_roles - {"b2_catalog", "b2_classifications", "b2_closure"}
    if invalid_b2:
        raise _fail(f"unknown B2 dependency role(s): {sorted(invalid_b2)!r}")
    if "b2_classifications" in dependency_roles:
        required_b2 = ("b2_catalog", "b2_classifications", "b2_closure")
    elif "b2_catalog" in dependency_roles:
        required_b2 = ("b2_catalog", "b2_closure")
    elif "b2_closure" in dependency_roles:
        required_b2 = ("b2_closure",)
    else:
        required_b2 = ()
    for role in required_b2:
        _require_role(by_role, role)

    if b1_citation:
        _require_role(by_role, "b1_final_citations")
        _require_role(by_role, "b1_final_closure")

    return canonical_source_bindings(
        tuple(seed_bindings)
        + tuple(by_role[role] for role in required_b2)
        + tuple(by_role[role] for role in ("b1_final_citations", "b1_final_closure") if b1_citation)
    )


def reconstruct_container_source_closure(
    *,
    static_bindings: Sequence[ContextAuthoritySourceBindingV2],
    event_leaf_bindings: Sequence[ContextAuthoritySourceBindingV2],
    event_closures: Sequence[Sequence[ContextAuthoritySourceBindingV2]],
    host_bindings: Sequence[ContextAuthoritySourceBindingV2] = (),
) -> tuple[ContextAuthoritySourceBindingV2, ...]:
    """Reconstruct one container's exact union of static/event dependencies."""

    for binding in tuple(static_bindings) + tuple(host_bindings):
        if binding.artifact_role == "context_application_authority_v2":
            raise _fail("container closure contains the forbidden context role")
    for closure in event_closures:
        for binding in closure:
            if binding.artifact_role in {
                "acceptance_event_leaf_v3",
                "context_application_authority_v2",
            }:
                raise _fail("event closure contains a forbidden cycle role")
    for binding in event_leaf_bindings:
        if binding.artifact_role != "acceptance_event_leaf_v3":
            raise _fail("container event leaf has the wrong source role")
    return canonical_source_bindings(
        tuple(static_bindings)
        + tuple(event_leaf_bindings)
        + tuple(binding for closure in event_closures for binding in closure)
        + tuple(host_bindings)
    )


def require_exact_source_set(
    actual: Sequence[ContextAuthoritySourceBindingV2],
    expected: Sequence[ContextAuthoritySourceBindingV2],
) -> None:
    """Reject any missing, extra, duplicate, or differently bound source."""

    actual_values = tuple(actual)
    actual_canonical = canonical_source_bindings(actual_values, reject_duplicates=True)
    if actual_values != actual_canonical:
        raise _fail("actual V2 source bindings are not in canonical order")
    expected_canonical = canonical_source_bindings(expected)
    if actual_canonical != expected_canonical:
        raise _fail("actual V2 source bindings do not equal the expected exact closure")


def _hex_digest(value: object, label: str) -> bytes:
    if not isinstance(value, str) or re.fullmatch(r"[0-9a-f]{64}", value) is None:
        raise _fail(f"{label} must be lowercase SHA-256 hex")
    return bytes.fromhex(value)


def _exact(value: object, keys: set[str], label: str) -> dict[str, object]:
    if not isinstance(value, Mapping) or set(value) != keys:
        raise _fail(f"{label} fields are not exactly {sorted(keys)!r}")
    return {cast(str, key): child for key, child in value.items()}


def context_source_binding_from_wire(value: object) -> ContextAuthoritySourceBindingV2:
    record = _exact(value, {"artifact_role", "path", "schema", "raw_sha256"}, "V2 source binding")
    try:
        binding = ContextAuthoritySourceBindingV2(
            artifact_role=cast(str, record["artifact_role"]),
            path=cast(str, record["path"]),
            schema=cast(str | None, record["schema"]),
            raw_sha256=_hex_digest(record["raw_sha256"], "V2 source binding raw_sha256"),
        )
        _validate_binding(binding)
        return binding
    except (TypeError, ValueError) as exc:
        raise _fail(str(exc)) from exc


class ContextApplicationV2Resolver:
    """Resolve exact V2 source bytes while remaining semantically read-only."""

    def __init__(
        self,
        resolver: AuthoritySourceResolver,
        *,
        base_authority_binding: ContextAuthoritySourceBindingV2 | None = None,
    ) -> None:
        self._resolver = resolver
        self._base_authority_binding = base_authority_binding

    def source_binding_for_evidence(
        self, reference: EvidenceRefV1
    ) -> ContextAuthoritySourceBindingV2:
        if not isinstance(reference, EvidenceRefV1):
            raise _fail("V2 source-binding mapping requires EvidenceRefV1")
        path = reference.path
        authority_kind = reference.authority_kind
        candidates: list[tuple[str, ContextAuthoritySourceRegistryEntryV2]] = []
        for role, entry in _registry().items():
            if re.fullmatch(entry.path_pattern, path) is not None:
                candidates.append((role, entry))
        if len(candidates) != 1:
            raise _fail(f"evidence path does not map to exactly one V2 source role: {path!r}")
        role, entry = candidates[0]
        if authority_kind is not None:
            expected_roles = {
                "model": {"declared_model"},
                "c_candidate": {"candidate_universe"},
                "rev3": {
                    candidate[0]
                    for candidate in candidates
                    if candidate[1].source_kind == "rev3_archive"
                },
                "b2": {"b2_catalog", "b2_classifications", "b2_closure"},
                "b1_final": {"b1_final_citations", "b1_final_closure"},
                "reviewer_roster": {"reviewer_roster_leaf"},
            }.get(cast(str, authority_kind))
            if expected_roles is None or role not in expected_roles:
                raise _fail(f"evidence authority/path is not admitted: {authority_kind}/{path}")
        try:
            binding = ContextAuthoritySourceBindingV2(
                artifact_role=role,
                path=path,
                schema=entry.schema,
                raw_sha256=reference.raw_sha256,
            )
            _validate_binding(binding)
            return binding
        except (TypeError, ValueError) as exc:
            raise _fail(str(exc)) from exc

    def resolve_source_binding(self, binding: ContextAuthoritySourceBindingV2) -> ResolvedArtifact:
        entry = _validate_binding(binding)
        if entry.source_kind == "rev3_archive":
            return self._resolver.resolve_rev3_member(
                binding.path, binding.raw_sha256, binding.schema
            )
        return self._resolver.resolve_repository_artifact(
            binding.path, binding.raw_sha256, binding.schema
        )

    def resolve_evidence(self, reference: EvidenceRefV1) -> ResolvedContextEvidence:
        binding = self.source_binding_for_evidence(reference)
        artifact = self.resolve_source_binding(binding)
        locator = reference.locator
        kind, payload = locator
        if kind == "archive_member":
            if binding.path != payload:
                raise _fail("archive-member evidence locator is not the bound source path")
            if _registry_entry(binding.artifact_role, binding.path, binding.schema).source_kind != (
                "rev3_archive"
            ):
                raise _fail("archive-member locator is only valid for REV3 source bindings")
            resolved = self._resolver.resolve_rev3_locator(
                locator, binding.raw_sha256, binding.schema
            )
            return ResolvedContextEvidence(binding, resolved.artifact, locator, resolved.value)
        if kind == "json_pointer":
            resolved = self._resolver.resolve_locator(artifact, locator)
            return ResolvedContextEvidence(binding, resolved.artifact, locator, resolved.value)
        if kind == "whole_artifact":
            value = artifact.json_value if artifact.json_value is not None else artifact.raw_bytes
            return ResolvedContextEvidence(binding, artifact, locator, value)
        raise _fail(f"unsupported evidence locator kind: {kind!r}")

    @staticmethod
    def _acceptance_json_pointer(value: object, pointer: str) -> object:
        if pointer == "":
            return value
        if not pointer.startswith("/"):
            raise _fail(
                "acceptance JSON Pointer must begin with '/'",
                code="REVIEW_EVIDENCE_INVALID",
            )
        current = value
        for raw_token in pointer[1:].split("/"):
            token = raw_token.replace("~1", "/").replace("~0", "~")
            if isinstance(current, Mapping):
                if token not in current:
                    raise _fail(
                        "acceptance JSON Pointer token is absent",
                        code="REVIEW_EVIDENCE_INVALID",
                    )
                current = current[token]
            elif isinstance(current, list):
                if not token.isdigit() or (token != "0" and token.startswith("0")):
                    raise _fail(
                        "acceptance JSON Pointer array index is invalid",
                        code="REVIEW_EVIDENCE_INVALID",
                    )
                index = int(token)
                if index >= len(current):
                    raise _fail(
                        "acceptance JSON Pointer index is out of range",
                        code="REVIEW_EVIDENCE_INVALID",
                    )
                current = current[index]
            else:
                raise _fail(
                    "acceptance JSON Pointer traverses a scalar",
                    code="REVIEW_EVIDENCE_INVALID",
                )
        return current

    def resolve_acceptance_evidence(
        self, evidence: AcceptanceEvidenceRefV1
    ) -> ResolvedContextEvidence:
        """Resolve review evidence without promoting it to a source binding."""

        if not isinstance(evidence, AcceptanceEvidenceRefV1):
            raise _fail(
                "acceptance evidence resolution requires AcceptanceEvidenceRefV1",
                code="REVIEW_EVIDENCE_INVALID",
            )
        artifact = self._resolver.resolve_repository_artifact(
            evidence.path, evidence.raw_sha256, None
        )
        kind, payload = evidence.locator
        if kind == "whole_artifact":
            value = artifact.raw_bytes
        elif kind == "json_pointer":
            try:
                parsed = json.loads(artifact.raw_bytes.decode("utf-8"))
            except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                raise _fail(
                    f"acceptance evidence is not UTF-8 JSON: {exc}",
                    code="REVIEW_EVIDENCE_INVALID",
                ) from exc
            value = self._acceptance_json_pointer(parsed, cast(str, payload))
        elif kind == "archive_member":
            try:
                with zipfile.ZipFile(io.BytesIO(artifact.raw_bytes)) as archive:
                    member = cast(str, payload)
                    if member not in archive.namelist():
                        raise _fail(
                            "acceptance evidence archive member is missing",
                            code="REVIEW_EVIDENCE_INVALID",
                        )
                    value = archive.read(member)
            except (OSError, zipfile.BadZipFile) as exc:
                raise _fail(
                    f"acceptance evidence archive member is unreadable: {exc}",
                    code="REVIEW_EVIDENCE_INVALID",
                ) from exc
        else:
            raise _fail(
                f"unsupported acceptance evidence locator kind: {kind!r}",
                code="REVIEW_EVIDENCE_INVALID",
            )
        return ResolvedContextEvidence(None, artifact, evidence.locator, value)

    @staticmethod
    def _digest_reference(value: object) -> DigestReferenceV1:
        try:
            record = _exact(
                value,
                {
                    "envelope_id",
                    "algorithm_id",
                    "semantic_domain",
                    "payload_codec_id",
                    "input_schema_id",
                    "digest_hex",
                },
                "V3 subject payload digest",
            )
            return DigestReferenceV1(
                envelope_id=cast(str, record["envelope_id"]),
                algorithm_id=cast(str, record["algorithm_id"]),
                semantic_domain=cast(str, record["semantic_domain"]),
                payload_codec_id=cast(str, record["payload_codec_id"]),
                input_schema_id=cast(str, record["input_schema_id"]),
                digest_bytes=_hex_digest(record["digest_hex"], "subject payload digest"),
            )
        except (TypeError, ValueError) as exc:
            raise _fail(str(exc), code="V3_EVENT_SCHEMA_INVALID") from exc

    @staticmethod
    def _locator(value: object) -> Locator:
        record = cast(dict[str, object], _exact(value, {"kind", "value"}, "locator"))
        kind = record["kind"]
        if kind == "whole_artifact":
            raise _fail("whole_artifact locator must omit value")
        return cast(Locator, (kind, record["value"]))

    @staticmethod
    def _acceptance_locator(value: object) -> Locator:
        if not isinstance(value, Mapping):
            raise _fail(
                "acceptance locator must be an object",
                code="REVIEW_EVIDENCE_INVALID",
            )
        kind = value.get("kind")
        if kind == "whole_artifact":
            if set(value) != {"kind"}:
                raise _fail(
                    "whole_artifact locator has an unexpected value",
                    code="REVIEW_EVIDENCE_INVALID",
                )
            return ("whole_artifact", None)
        if kind in {"json_pointer", "archive_member"} and set(value) == {"kind", "value"}:
            return cast(Locator, (kind, value["value"]))
        raise _fail(
            "V3 acceptance locator is not closed",
            code="REVIEW_EVIDENCE_INVALID",
        )

    def resolve_review_event_leaf_v3(
        self, reference: ReviewEventRefV3
    ) -> ResolvedReviewAcceptanceEventV3:
        """Resolve only the raw/structural V3 leaf required by container closure."""

        if not isinstance(reference, ReviewEventRefV3):
            raise _fail(
                "V3 review event reference has the wrong type",
                code="V3_EVENT_REFERENCE_INVALID",
            )
        binding = ContextAuthoritySourceBindingV2(
            "acceptance_event_leaf_v3",
            reference.path,
            ACCEPTANCE_EVENT_SCHEMA_V3,
            reference.raw_sha256,
        )
        artifact = self.resolve_source_binding(binding)
        if artifact.json_value is None or not isinstance(artifact.json_value, Mapping):
            raise _fail(
                "V3 acceptance event leaf is not a JSON object",
                code="V3_EVENT_SCHEMA_INVALID",
            )
        try:
            record = _exact(
                artifact.json_value,
                {
                    "event_id",
                    "schema",
                    "subject_kind",
                    "subject_payload_digest",
                    "decision",
                    "reviewer_roster_ref",
                    "reviewer_role_bindings",
                    "review_mode",
                    "checklist_id",
                    "source_binding_digests",
                    "review_evidence_refs",
                },
                "V3 acceptance event leaf",
            )
        except ContextApplicationV2ResolutionError as exc:
            raise _fail(str(exc), code="V3_EVENT_SCHEMA_INVALID") from exc
        if record["event_id"] != reference.event_id:
            raise _fail(
                "V3 acceptance event ID differs from its reference",
                code="V3_EVENT_IDENTITY_INVALID",
            )
        if record["schema"] != ACCEPTANCE_EVENT_SCHEMA_V3:
            raise _fail(
                "V3 acceptance event schema is not V3",
                code="V3_EVENT_SCHEMA_INVALID",
            )
        if record["decision"] != "human_accepted":
            raise _fail(
                "V3 acceptance event decision is not human_accepted",
                code="V3_EVENT_DECISION_INVALID",
            )
        if record["checklist_id"] != "interaction-authority-review-checklist.v2":
            raise _fail(
                "V3 acceptance event checklist is not V2",
                code="CHECKLIST_V2_MISMATCH",
            )
        raw_review_mode = record["review_mode"]
        try:
            review_mode = ReviewMode(cast(str, raw_review_mode))
        except (TypeError, ValueError) as exc:
            raise _fail(
                f"V3 acceptance event structural fields are invalid: {exc}",
                code="REVIEW_MODE_INVALID",
            ) from exc

        raw_sources = record["source_binding_digests"]
        if not isinstance(raw_sources, list):
            raise _fail(
                "V3 acceptance event structural fields are invalid: V3 source binding digests must be an array",
                code="V3_EVENT_SOURCE_INVALID",
            )
        raw_evidence = record["review_evidence_refs"]
        if not isinstance(raw_evidence, list):
            raise _fail(
                "V3 acceptance event structural fields are invalid: V3 review evidence refs must be an array",
                code="REVIEW_EVIDENCE_INVALID",
            )
        if not raw_evidence:
            raise _fail(
                "V3 acceptance event structural fields are invalid: V3 review evidence must be non-empty",
                code="REVIEW_EVIDENCE_MISSING",
            )

        try:
            subject_kind = AcceptanceSubjectKindV3(cast(str, record["subject_kind"]))
            subject_digest = self._digest_reference(record["subject_payload_digest"])
            roster_record = _exact(
                record["reviewer_roster_ref"],
                {"path", "schema", "raw_sha256"},
                "V3 reviewer roster reference",
            )
            roster_ref = ReviewerRosterRefV1(
                cast(str, roster_record["path"]),
                cast(str, roster_record["schema"]),
                _hex_digest(roster_record["raw_sha256"], "reviewer roster digest"),
            )
            raw_roles = record["reviewer_role_bindings"]
            if not isinstance(raw_roles, list):
                raise ValueError("V3 reviewer role bindings must be an array")
            role_bindings = tuple(
                ReviewerRoleBindingV1(
                    cast(str, cast(Mapping[str, object], item)["reviewer_id"]),
                    tuple(cast(list[str], cast(Mapping[str, object], item)["roles"])),
                )
                for item in raw_roles
            )
            try:
                sources = tuple(context_source_binding_from_wire(item) for item in raw_sources)
            except ContextApplicationV2ResolutionError as exc:
                raise _fail(
                    f"V3 acceptance event structural fields are invalid: {exc}",
                    code="V3_EVENT_SOURCE_INVALID",
                ) from exc
            source_keys = [(item.artifact_role, item.path) for item in sources]
            if len(source_keys) != len(set(source_keys)):
                raise _fail(
                    "V3 source bindings must not duplicate a role/path pair",
                    code="V3_EVENT_SOURCE_INVALID",
                )
            source_order = [encode_canonical(item.to_cbor()) for item in sources]
            if source_order != sorted(source_order):
                raise _fail(
                    "V3 source bindings are not in canonical order",
                    code="V3_EVENT_SOURCE_INVALID",
                )
            evidence = tuple(
                AcceptanceEvidenceRefV1(
                    path=cast(str, cast(Mapping[str, object], item)["path"]),
                    raw_sha256=_hex_digest(
                        cast(Mapping[str, object], item)["raw_sha256"],
                        "review evidence digest",
                    ),
                    locator=self._acceptance_locator(cast(Mapping[str, object], item)["locator"]),
                )
                for item in raw_evidence
            )
            event = ReviewAcceptanceEventInputV3(
                subject_kind=subject_kind,
                subject_payload_digest_reference=subject_digest,
                reviewer_roster_ref=roster_ref,
                reviewer_role_bindings=role_bindings,
                review_mode=review_mode,
                source_binding_digests=sources,
                review_evidence_refs=evidence,
            )
        except ContextApplicationV2ResolutionError as exc:
            raise _fail(
                f"V3 acceptance event structural fields are invalid: {exc}",
                code=exc.code or "V3_EVENT_SCHEMA_INVALID",
            ) from exc
        except (TypeError, ValueError) as exc:
            raise _fail(
                f"V3 acceptance event structural fields are invalid: {exc}",
                code="V3_EVENT_SCHEMA_INVALID",
            ) from exc

        try:
            recomputed_event_id = event.identity().as_text()
        except (TypeError, ValueError) as exc:
            raise _fail(
                f"V3 acceptance event identity cannot be recomputed: {exc}",
                code="V3_EVENT_IDENTITY_INVALID",
            ) from exc
        if recomputed_event_id != reference.event_id:
            raise _fail(
                "V3 acceptance event semantic ID differs from its reference",
                code="V3_EVENT_IDENTITY_INVALID",
            )

        roster_binding = ContextAuthoritySourceBindingV2(
            "reviewer_roster_leaf", roster_ref.path, roster_ref.schema, roster_ref.raw_sha256
        )
        if roster_binding not in event.source_binding_digests:
            raise _fail(
                "V3 event does not bind its exact reviewer roster leaf",
                code="V3_EVENT_SOURCE_INVALID",
            )
        if any(
            item.artifact_role == "acceptance_event_leaf_v3" and item.path == reference.path
            for item in event.source_binding_digests
        ):
            raise _fail(
                "V3 event source list contains its own acceptance leaf",
                code="V3_EVENT_SOURCE_INVALID",
            )
        for evidence in event.review_evidence_refs:
            self.resolve_acceptance_evidence(evidence)
        return ResolvedReviewAcceptanceEventV3(
            reference=reference,
            artifact=artifact,
            event_id=reference.event_id,
            event=event,
        )

    def _base_binding(
        self, override: ContextAuthoritySourceBindingV2 | None
    ) -> ContextAuthoritySourceBindingV2:
        binding = override or self._base_authority_binding
        if binding is None:
            raise _fail("base_authority_v1 binding is required for V2 closure reconstruction")
        if binding.artifact_role != "base_authority_v1":
            raise _fail("base closure binding must use role=base_authority_v1")
        _validate_binding(binding)
        return binding

    def _base_context(
        self, override: ContextAuthoritySourceBindingV2 | None
    ) -> tuple[
        ContextAuthoritySourceBindingV2,
        ContextAuthoritySourceBindingV2,
        tuple[ContextAuthoritySourceBindingV2, ...],
        Mapping[str, object],
    ]:
        base_binding = self._base_binding(override)
        artifact = self.resolve_source_binding(base_binding)
        if not isinstance(artifact.json_value, Mapping):
            raise _fail("base authority V1 source is not a JSON object")
        AuthorityValidator(self._resolver).validate(dict(artifact.json_value))
        model_record = _exact(
            artifact.json_value.get("model_binding"),
            {"path", "raw_sha256", "model_id", "model_version"},
            "base authority model binding",
        )
        model_binding = ContextAuthoritySourceBindingV2(
            "declared_model",
            cast(str, model_record["path"]),
            _registry()["declared_model"].schema,
            _hex_digest(model_record["raw_sha256"], "base model digest"),
        )
        available: list[ContextAuthoritySourceBindingV2] = [model_binding]
        raw_sources = artifact.json_value.get("source_bindings", [])
        if not isinstance(raw_sources, list):
            raise _fail("base authority source_bindings must be an array")
        for raw_source in raw_sources:
            if not isinstance(raw_source, Mapping):
                raise _fail("base authority source binding is not an object")
            path = raw_source.get("path")
            schema = raw_source.get("schema_or_null")
            raw_sha256 = raw_source.get("raw_sha256")
            if not isinstance(path, str) or not isinstance(schema, str | None):
                raise _fail("base authority source binding fields are invalid")
            matches = [
                (role, entry)
                for role, entry in _registry().items()
                if re.fullmatch(entry.path_pattern, path) is not None and entry.schema == schema
            ]
            if len(matches) != 1:
                continue
            role, _ = matches[0]
            available.append(
                ContextAuthoritySourceBindingV2(
                    role,
                    path,
                    schema,
                    _hex_digest(raw_sha256, "base source binding digest"),
                )
            )
        return (
            base_binding,
            model_binding,
            tuple(available),
            cast(dict[str, object], artifact.json_value),
        )

    @staticmethod
    def _evidence_from_wire(value: object) -> EvidenceRefV1:
        record = _exact(
            value,
            {"authority_kind", "path", "locator", "raw_sha256"},
            "V1 evidence reference",
        )
        locator_record = record["locator"]
        if not isinstance(locator_record, Mapping):
            raise _fail("V1 evidence locator is not an object")
        kind = locator_record.get("kind")
        if kind == "whole_artifact":
            if set(locator_record) != {"kind"}:
                raise _fail("whole_artifact evidence locator has an unexpected value")
            locator: Locator = ("whole_artifact", None)
        elif kind in {"json_pointer", "archive_member", "event_id"} and set(locator_record) == {
            "kind",
            "value",
        }:
            locator = cast(Locator, (kind, locator_record["value"]))
        else:
            raise _fail("V1 evidence locator is not closed")
        try:
            return EvidenceRefV1(
                authority_kind=cast(str, record["authority_kind"]),
                path=cast(str, record["path"]),
                locator=locator,
                raw_sha256=_hex_digest(record["raw_sha256"], "evidence digest"),
            )
        except (TypeError, ValueError) as exc:
            raise _fail(str(exc)) from exc

    def _collect_evidence(
        self,
        references: Iterable[EvidenceRefV1],
    ) -> tuple[tuple[ContextAuthoritySourceBindingV2, ...], tuple[str, ...], bool]:
        bindings: list[ContextAuthoritySourceBindingV2] = []
        b2_roles: set[str] = set()
        b1 = False
        for reference in references:
            resolved = self.resolve_evidence(reference)
            bindings.append(resolved.binding)
            if resolved.binding.artifact_role.startswith("b2_"):
                b2_roles.add(resolved.binding.artifact_role)
            if resolved.binding.artifact_role == "b1_final_citations":
                b1 = True
        return canonical_source_bindings(bindings), tuple(sorted(b2_roles)), b1

    @staticmethod
    def _evidence_from_cbor(value: object) -> EvidenceRefV1:
        if not isinstance(value, list) or len(value) != 4:
            raise _fail("V1 CBOR evidence reference is malformed")
        locator_value = value[2]
        if not isinstance(locator_value, list) or len(locator_value) != 2:
            raise _fail("V1 CBOR evidence locator is malformed")
        try:
            return EvidenceRefV1(
                authority_kind=cast(str, value[0]),
                path=cast(str, value[1]),
                locator=cast(Locator, (locator_value[0], locator_value[1])),
                raw_sha256=cast(bytes, value[3]),
            )
        except (TypeError, ValueError) as exc:
            raise _fail(str(exc)) from exc

    def _walk_v1_dependencies(
        self, value: object
    ) -> tuple[tuple[ContextAuthoritySourceBindingV2, ...], tuple[str, ...], bool]:
        evidence: list[EvidenceRefV1] = []
        b2_roles: set[str] = set()
        b1 = False

        def walk(node: object, key: str | None = None) -> None:
            nonlocal b1
            if key == "acceptance":
                return
            if key in {
                "source_evidence_refs",
                "member_evidence_refs",
                "evidence_refs",
                "positive_boundary_evidence_refs",
            }:
                if not isinstance(node, list):
                    raise _fail(f"{key} must be an array")
                for item in node:
                    if isinstance(item, Mapping):
                        evidence.append(self._evidence_from_wire(item))
                    else:
                        evidence.append(self._evidence_from_cbor(item))
                return
            if key in {"b2_boundary_refs", "b2_family_refs", "through_boundary_refs"}:
                if not isinstance(node, list):
                    raise _fail(f"{key} must be an array")
                if node:
                    b2_roles.add("b2_catalog")
                return
            if key == "b1_final_citation_refs":
                if not isinstance(node, list):
                    raise _fail("b1_final_citation_refs must be an array")
                if node:
                    b1 = True
                return
            if isinstance(node, Mapping):
                for child_key, child in node.items():
                    walk(child, cast(str, child_key))
            elif isinstance(node, list):
                for child in node:
                    walk(child, key)

        walk(value)
        direct, evidence_b2_roles, evidence_b1 = self._collect_evidence(evidence)
        b2_roles.update(evidence_b2_roles)
        return direct, tuple(sorted(b2_roles)), b1 or evidence_b1

    @staticmethod
    def _record_id_text(value: object, prefix: str) -> str:
        if not isinstance(value, Mapping):
            raise _fail("V1 record ID is not an object")
        digest = value.get("digest_hex")
        if not isinstance(digest, str) or re.fullmatch(r"[0-9a-f]{64}", digest) is None:
            raise _fail("V1 record ID has an invalid digest")
        return prefix + digest

    def _context_theorem_record(
        self, base_document: Mapping[str, object], theorem_record_id: str
    ) -> Mapping[str, object]:
        raw_records = base_document.get("context_proofs")
        if not isinstance(raw_records, list):
            raise _fail("base authority context_proofs is not an array")
        for raw_record in raw_records:
            if (
                isinstance(raw_record, Mapping)
                and self._record_id_text(raw_record.get("record_id"), "cpr.v1/")
                == theorem_record_id
            ):
                return raw_record
        raise _fail(f"context theorem record is absent: {theorem_record_id}")

    def _candidate_available_bindings(
        self, candidate_universe: object
    ) -> tuple[ContextAuthoritySourceBindingV2, ...]:
        if not isinstance(candidate_universe, Mapping):
            raise _fail("candidate universe is not a JSON object")
        raw_inputs = candidate_universe.get("input_bindings")
        if not isinstance(raw_inputs, Mapping):
            raise _fail("candidate universe input_bindings is not an object")
        result: list[ContextAuthoritySourceBindingV2] = []
        for key in ("b2_artifacts", "b1_final_artifacts"):
            raw_items = raw_inputs.get(key)
            if not isinstance(raw_items, list):
                continue
            for raw_item in raw_items:
                if not isinstance(raw_item, Mapping):
                    raise _fail(f"candidate universe {key} item is not an object")
                path = raw_item.get("path")
                if not isinstance(path, str):
                    raise _fail(f"candidate universe {key} path is invalid")
                matches = [
                    (role, entry)
                    for role, entry in _registry().items()
                    if re.fullmatch(entry.path_pattern, path) is not None
                ]
                if len(matches) != 1:
                    continue
                role, entry = matches[0]
                result.append(
                    ContextAuthoritySourceBindingV2(
                        role,
                        path,
                        entry.schema,
                        _hex_digest(raw_item.get("raw_sha256"), f"candidate universe {key} digest"),
                    )
                )
        return canonical_source_bindings(result)

    def resolve_member_source_instance(
        self,
        member: ContextApplicationMemberV2,
    ) -> ResolvedSourceInstance:
        """Resolve one V2 member through the authoritative Slice-2 path."""

        if not isinstance(member, ContextApplicationMemberV2):
            raise _fail("V2 member source resolution requires ContextApplicationMemberV2")
        binding_values = member.candidate_universe_binding
        if not isinstance(binding_values, list) or len(binding_values) != 3:
            raise _fail("V2 member candidate-universe binding is malformed")
        raw_path, raw_schema, raw_digest = binding_values
        if (
            not isinstance(raw_path, str)
            or not isinstance(raw_schema, str)
            or not isinstance(raw_digest, bytes)
            or len(raw_digest) != 32
        ):
            raise _fail("V2 member candidate-universe binding has invalid fields")
        binding = SourceBindingDigestV1(
            "candidate_universe",
            raw_path,
            raw_schema,
            raw_digest,
        )
        return self._resolver.resolve_candidate_source_instance(
            member.candidate_id,
            member.candidate_identity_digest_reference.to_wire(),
            member.source_instance_id,
            binding,
        )

    def _candidate_provenance(
        self, member: ContextApplicationMemberV2
    ) -> tuple[
        tuple[ContextAuthoritySourceBindingV2, ...],
        tuple[ContextAuthoritySourceBindingV2, ...],
    ]:
        resolved = self.resolve_member_source_instance(member)
        candidate_binding_v1 = resolved.candidate.candidate_universe_binding
        candidate_binding = ContextAuthoritySourceBindingV2(
            "candidate_universe",
            candidate_binding_v1.path,
            candidate_binding_v1.schema_or_null,
            candidate_binding_v1.raw_sha256,
        )
        source_matches = [
            (role, entry)
            for role, entry in _registry().items()
            if entry.source_kind == "rev3_archive"
            and re.fullmatch(entry.path_pattern, resolved.source_artifact.path) is not None
        ]
        if len(source_matches) != 1:
            raise _fail(
                "resolved REV3 source path is not a closed V2 source role: "
                f"{resolved.source_artifact.path}"
            )
        source_role, source_entry = source_matches[0]
        provenance = [
            ContextAuthoritySourceBindingV2(
                source_role,
                resolved.source_artifact.path,
                source_entry.schema,
                bytes.fromhex(resolved.source_artifact.raw_sha256),
            )
        ]
        candidate_record = resolved.candidate.candidate_record
        if candidate_record.get("relation") == "declared_card_trigger":
            for member_path in (
                "inputs/deck_row_source_resolution_REV3.csv",
                "source/raw/oracle_cards_selected_REV3.jsonl",
                "source/raw/source_record_index_REV3.csv",
            ):
                artifact = self._resolver.resolve_rev3_member_from_manifest(member_path)
                matches = [
                    (role, entry)
                    for role, entry in _registry().items()
                    if entry.source_kind == "rev3_archive"
                    and re.fullmatch(entry.path_pattern, member_path) is not None
                ]
                if len(matches) != 1:
                    raise _fail(f"REV3 provenance path is not a closed V2 role: {member_path}")
                role, entry = matches[0]
                provenance.append(
                    ContextAuthoritySourceBindingV2(
                        role,
                        member_path,
                        entry.schema,
                        bytes.fromhex(artifact.raw_sha256),
                    )
                )
        return (
            canonical_source_bindings((candidate_binding, *provenance)),
            self._candidate_available_bindings(resolved.candidate.candidate_universe.json_value),
        )

    @staticmethod
    def _member_evidence(member: ContextApplicationMemberV2) -> tuple[EvidenceRefV1, ...]:
        result: list[EvidenceRefV1] = list(member.member_evidence_refs)
        for slot in member.context_member_bridge_attestation_v2.context:
            result.extend(slot.evidence_refs)
        for slot in member.context_member_bridge_attestation_v2.temporal:
            result.extend(slot.evidence_refs)
        for index, precondition in enumerate(member.precondition_attestations_v1):
            if not isinstance(precondition, list) or len(precondition) != 4:
                raise _fail(f"member precondition {index} is malformed")
            raw_evidence = precondition[2]
            if not isinstance(raw_evidence, list):
                raise _fail(f"member precondition {index} evidence is malformed")
            result.extend(
                ContextApplicationV2Resolver._evidence_from_cbor(item) for item in raw_evidence
            )
        return tuple(result)

    def _expected_acceptance_source_closure_v3(
        self,
        subject: ContextApplicationV2Record | ContextApplicationV2SupersessionRecord,
        reviewer_roster_ref: ReviewerRosterRefV1,
        *,
        base_authority_binding: ContextAuthoritySourceBindingV2 | None = None,
        host_bindings: Sequence[ContextAuthoritySourceBindingV2] = (),
    ) -> tuple[ContextAuthoritySourceBindingV2, ...]:
        """Reconstruct a V3 event closure without reading its source list."""

        if not isinstance(reviewer_roster_ref, ReviewerRosterRefV1):
            raise _fail("reviewer roster reference has the wrong type")
        base_binding, model_binding, available, base_document = self._base_context(
            base_authority_binding
        )
        roster_binding = ContextAuthoritySourceBindingV2(
            "reviewer_roster_leaf",
            reviewer_roster_ref.path,
            reviewer_roster_ref.schema,
            reviewer_roster_ref.raw_sha256,
        )
        direct: list[ContextAuthoritySourceBindingV2] = []
        b2_roles: set[str] = set()
        b1 = False

        if isinstance(subject, ContextApplicationV2SupersessionRecord):
            evidence_bindings, evidence_b2_roles, evidence_b1 = self._collect_evidence(
                subject.source_evidence_refs
            )
            direct.extend(evidence_bindings)
            b2_roles.update(evidence_b2_roles)
            b1 = b1 or evidence_b1
        elif isinstance(subject, ContextApplicationV2Record):
            for member in subject.members:
                candidate_bindings, candidate_available = self._candidate_provenance(member)
                direct.extend(candidate_bindings)
                available = tuple(available) + tuple(candidate_available)
                evidence_bindings, evidence_b2_roles, evidence_b1 = self._collect_evidence(
                    self._member_evidence(member)
                )
                direct.extend(evidence_bindings)
                b2_roles.update(evidence_b2_roles)
                b1 = b1 or evidence_b1
            theorem_record = self._context_theorem_record(
                base_document, subject.theorem_record_id.as_text()
            )
            theorem_bindings, theorem_b2_roles, theorem_b1 = self._walk_v1_dependencies(
                theorem_record
            )
            direct.extend(theorem_bindings)
            b2_roles.update(theorem_b2_roles)
            b1 = b1 or theorem_b1
        else:
            raise _fail("V2 acceptance subject has an unsupported type")

        expected = reconstruct_event_source_closure(
            fixed_bindings=(base_binding, model_binding, roster_binding),
            direct_bindings=tuple(direct),
            available_bindings=available,
            b2_evidence_roles=b2_roles,
            b1_citation=b1,
            host_bindings=host_bindings,
        )
        for binding in expected:
            self.resolve_source_binding(binding)
        return expected

    def expected_acceptance_source_closure_v3(
        self,
        subject: ContextApplicationV2Record | ContextApplicationV2SupersessionRecord,
        reviewer_roster_ref: ReviewerRosterRefV1,
        *,
        base_authority_binding: ContextAuthoritySourceBindingV2 | None = None,
    ) -> tuple[ContextAuthoritySourceBindingV2, ...]:
        """Reconstruct a standalone event closure without caller-selected host sources."""

        return self._expected_acceptance_source_closure_v3(
            subject,
            reviewer_roster_ref,
            base_authority_binding=base_authority_binding,
        )

    @staticmethod
    def _validate_container_projections(
        container: object,
    ) -> tuple[ContextAuthoritySourceBindingV2, ...]:
        if not hasattr(container, "source_bindings"):
            raise _fail("V2 container has no source_bindings projection")
        source_bindings = cast(
            tuple[ContextAuthoritySourceBindingV2, ...], container.source_bindings
        )
        canonical_source_bindings(source_bindings, reject_duplicates=True)
        by_role: dict[str, list[ContextAuthoritySourceBindingV2]] = {}
        for binding in source_bindings:
            by_role.setdefault(binding.artifact_role, []).append(binding)

        def require_projection(
            field_name: str,
            role: str,
            expected: ContextAuthoritySourceBindingV2 | None,
        ) -> None:
            matches = by_role.get(role, [])
            if expected is None:
                if matches:
                    raise _fail(f"container has an unexpected {role} source projection")
                return
            if len(matches) != 1 or matches[0] != expected:
                raise _fail(f"container {field_name} projection is not exact")

        require_projection(
            "base_authority_v1_binding",
            "base_authority_v1",
            cast(ContextAuthoritySourceBindingV2, container.base_authority_v1_binding),
        )
        require_projection(
            "candidate_universe_binding",
            "candidate_universe",
            cast(ContextAuthoritySourceBindingV2, container.candidate_universe_binding),
        )
        require_projection(
            "host_binding_authority_v2_binding",
            "host_binding_authority_v2",
            cast(
                ContextAuthoritySourceBindingV2 | None,
                container.host_binding_authority_v2_binding,
            ),
        )
        return source_bindings

    @staticmethod
    def _host_bindings_for_claim_ids(
        claim_ids: Iterable[str],
        host_authority: ContextAuthoritySourceBindingV2 | None,
        source_bindings: Sequence[ContextAuthoritySourceBindingV2],
    ) -> tuple[ContextAuthoritySourceBindingV2, ...]:
        if claim_ids and host_authority is None:
            raise _fail("host-binding application links require host_binding_authority_v2")
        required: list[ContextAuthoritySourceBindingV2] = []
        if host_authority is not None and claim_ids:
            required.append(host_authority)
        for claim_id in sorted(set(claim_ids)):
            if re.fullmatch(r"hbc\.v1/[0-9a-f]{64}", claim_id) is None:
                raise _fail("V2 host-binding claim ID is not closed")
            claim_path = (
                "sources/m2_5/authorities/cross_deck_host_binding_claims/v1/"
                + claim_id.removeprefix("hbc.v1/")
                + ".json"
            )
            matches = [
                binding
                for binding in source_bindings
                if binding.artifact_role == "host_binding_claim_record"
                and binding.path == claim_path
            ]
            if len(matches) != 1:
                raise _fail(f"missing host-binding claim source: {claim_path}")
            required.append(matches[0])
        return canonical_source_bindings(required)

    @classmethod
    def _container_host_bindings(
        cls,
        container: object,
        source_bindings: Sequence[ContextAuthoritySourceBindingV2],
    ) -> tuple[ContextAuthoritySourceBindingV2, ...]:
        host_authority = cast(
            ContextAuthoritySourceBindingV2 | None,
            container.host_binding_authority_v2_binding,
        )
        links = cast(tuple[object, ...], container.application_host_bindings_v2)
        if links and host_authority is None:
            raise _fail("host-binding application links require host_binding_authority_v2")
        if not links and host_authority is not None:
            return (host_authority,)
        claim_ids: set[str] = set()
        for link in links:
            raw_claim_ids = getattr(link, "host_binding_claim_ids", None)
            if not isinstance(raw_claim_ids, tuple):
                raise _fail("V2 host-binding claim projection is malformed")
            claim_ids.update(cast(tuple[str, ...], raw_claim_ids))
        return cls._host_bindings_for_claim_ids(claim_ids, host_authority, source_bindings)

    @classmethod
    def _container_host_bindings_for_application(
        cls,
        container: object,
        application_semantic_id: str,
        source_bindings: Sequence[ContextAuthoritySourceBindingV2],
    ) -> tuple[ContextAuthoritySourceBindingV2, ...]:
        links = tuple(
            link
            for link in cast(tuple[object, ...], container.application_host_bindings_v2)
            if getattr(getattr(link, "application_semantic_id", None), "as_text", lambda: None)()
            == application_semantic_id
        )
        claim_ids: list[str] = []
        for link in links:
            raw_claim_ids = getattr(link, "host_binding_claim_ids", None)
            if not isinstance(raw_claim_ids, tuple):
                raise _fail("V2 host-binding claim projection is malformed")
            claim_ids.extend(cast(tuple[str, ...], raw_claim_ids))
        return cls._host_bindings_for_claim_ids(
            claim_ids,
            cast(
                ContextAuthoritySourceBindingV2 | None,
                container.host_binding_authority_v2_binding,
            ),
            source_bindings,
        )

    def expected_container_source_closure_v2(
        self, container: object
    ) -> tuple[ContextAuthoritySourceBindingV2, ...]:
        """Reconstruct the container closure and independently validate event closures."""

        source_bindings = self._validate_container_projections(container)
        host_bindings = self._container_host_bindings(container, source_bindings)
        event_leaf_bindings: list[ContextAuthoritySourceBindingV2] = []
        event_closures: list[tuple[ContextAuthoritySourceBindingV2, ...]] = []
        records = tuple(container.context_application_v2_records) + tuple(
            container.context_application_v2_supersession_records
        )
        for record in records:
            reference = record.review_event_ref_v3
            event_leaf = ContextAuthoritySourceBindingV2(
                "acceptance_event_leaf_v3",
                reference.path,
                ACCEPTANCE_EVENT_SCHEMA_V3,
                reference.raw_sha256,
            )
            resolved_event = self.resolve_review_event_leaf_v3(reference)
            event_host_bindings = (
                self._container_host_bindings_for_application(
                    container,
                    record.application_id.as_text(),
                    source_bindings,
                )
                if isinstance(record, ContextApplicationV2Record)
                else ()
            )
            expected_event = self._expected_acceptance_source_closure_v3(
                record,
                resolved_event.event.reviewer_roster_ref,
                base_authority_binding=container.base_authority_v1_binding,
                host_bindings=event_host_bindings,
            )
            require_exact_source_set(resolved_event.event.source_binding_digests, expected_event)
            event_leaf_bindings.append(event_leaf)
            event_closures.append(expected_event)

        expected = reconstruct_container_source_closure(
            static_bindings=(
                container.base_authority_v1_binding,
                container.candidate_universe_binding,
            ),
            event_leaf_bindings=tuple(event_leaf_bindings),
            event_closures=tuple(event_closures),
            host_bindings=host_bindings,
        )
        for binding in expected:
            self.resolve_source_binding(binding)
        return expected

    def validate_event_source_closure_v3(
        self,
        subject: ContextApplicationV2Record | ContextApplicationV2SupersessionRecord,
        event: ResolvedReviewAcceptanceEventV3,
        *,
        base_authority_binding: ContextAuthoritySourceBindingV2 | None = None,
    ) -> tuple[ContextAuthoritySourceBindingV2, ...]:
        expected = self.expected_acceptance_source_closure_v3(
            subject,
            event.event.reviewer_roster_ref,
            base_authority_binding=base_authority_binding,
        )
        require_exact_source_set(event.event.source_binding_digests, expected)
        return expected

    def validate_container_source_closure_v2(
        self, container: object
    ) -> tuple[ContextAuthoritySourceBindingV2, ...]:
        expected = self.expected_container_source_closure_v2(container)
        require_exact_source_set(container.source_bindings, expected)
        return expected


__all__ = [
    "ContextApplicationV2ResolutionError",
    "ContextApplicationV2Resolver",
    "ResolvedContextEvidence",
    "ResolvedReviewAcceptanceEventV3",
    "canonical_source_bindings",
    "context_source_binding_from_wire",
    "reconstruct_container_source_closure",
    "reconstruct_event_source_closure",
    "require_exact_source_set",
]
