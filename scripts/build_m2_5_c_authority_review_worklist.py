"""Build a deterministic, non-authoritative M2.5.C review worklist.

This module consumes source artifacts through ``AuthoritySourceResolver`` and
only projects facts already present in the accepted C candidate/classification
sources.  It never creates an accepted authority record or evaluates Magic
semantics.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Final, NoReturn, TypeAlias, cast

from authority_source_resolver import (
    CANDIDATE_UNIVERSE_SCHEMA,
    DECLARED_MODEL_SCHEMA,
    AuthoritySourceResolver,
    ResolutionError,
)
from mtgml.authority import (
    REVIEWER_ROSTER_SCHEMA_V1,
    ReviewerRosterRefV1,
    SourceBindingDigestV1,
)

ROOT = Path(__file__).resolve().parents[1]
C_DIR = "sources/m2_5/closures/C"
MODEL_PATH = f"{C_DIR}/declared_interaction_model.v2.json"
CANDIDATE_UNIVERSE_PATH = f"{C_DIR}/interaction_candidate_universe.v2.json"
CLOSURE_PATH = f"{C_DIR}/interaction_closure.v3.json"
SEMANTIC_CLASSES_PATH = f"{C_DIR}/interaction_semantic_classes.v2.json"
CLASSIFICATION_ROOT_PATH = f"{C_DIR}/interaction_classifications.v3.json"
ROSTER_DIRECTORY = "sources/m2_5/authorities/reviewer_rosters/v1"
PRODUCTION_ROSTER_DIGEST = "6238d8ff880460adddacc8f1c79ae972d0db150ae19b5ea636431d3f4e90cd36"
PRODUCTION_ROSTER_PATH = f"{ROSTER_DIRECTORY}/{PRODUCTION_ROSTER_DIGEST}.json"
CLASSIFICATION_ROOT_SCHEMA = "manafold.m2.5.c.interaction-classifications.v3"
CLASSIFICATION_SHARD_SCHEMA = "manafold.m2.5.c.interaction-classifications-shard.v3"
CLOSURE_SCHEMA = "manafold.m2.5.c.interaction-closure.v3"
SEMANTIC_CLASSES_SCHEMA = "manafold.m2.5.c.interaction-semantic-classes.v2"
WORKLIST_FORMAT = "manafold.m2.5.c.authority-review-worklist.v1"
PROPOSAL_FORMAT = "manafold.m2.5.c.authority-review-proposal.v1"
WORKLIST_NAME = "review_worklist.v1.jsonl"
SUMMARY_NAME = "REVIEW_WORKLIST_SUMMARY.md"
PARTITION_SCHEME = "candidate-order-fixed-chunk-1000-v1"
REVIEW_STATE_UNRESOLVED = "unresolved"
# These are the accepted C snapshot identities at the Task 5 Slice 2 base.
# The worklist is intentionally tied to that unresolved source universe rather
# than silently adapting to a later or locally rewritten C snapshot.
ACCEPTED_CLOSURE_SHA256: Final = "df771174462a5a4878c50b8b47e5d70f82dab1ba2cf9432a4998a7acc9f074e4"
ACCEPTED_MODEL_SHA256: Final = "8a7cb9a3b48468a097741e8b426977e443f7fa513705d56d213e467311bbf524"
ACCEPTED_CANDIDATE_UNIVERSE_SHA256: Final = (
    "1f8761af56f8b44c5e51d8cb9fcff79dd95dd56a98bfc6793e2ca8860050c532"
)
ACCEPTED_SEMANTIC_CLASSES_SHA256: Final = (
    "b1ad1780a04b03944335689674dd9fd1a91c2c52c486cd1863c3e6a8e5e1bc35"
)
ACCEPTED_CLASSIFICATION_ROOT_SHA256: Final = (
    "e0bb62299cc3d10395a15257dc10a7501f8b6e87f86e6e8c1e77511f2093616f"
)
CLASSIFICATION_ROOT_KEYS: Final = {
    "candidate_universe_raw_sha256",
    "classification_count",
    "model_id",
    "partition_scheme",
    "schema",
    "semantic_classes_raw_sha256",
    "shard_count",
    "shards",
}
CLASSIFICATION_SHARD_KEYS: Final = {
    "candidate_classifications",
    "candidate_universe_raw_sha256",
    "first_candidate_id",
    "last_candidate_id",
    "model_id",
    "ordinal_end_exclusive",
    "ordinal_start",
    "partition_scheme",
    "record_count",
    "schema",
    "semantic_classes_raw_sha256",
    "shard_count",
    "shard_index",
}
CLASSIFICATION_RECORD_KEYS: Final = {
    "candidate_id",
    "evidence_refs",
    "interaction_class_id",
    "reconciliation",
    "review_domain_assessments",
    "review_rationale",
    "review_state",
    "source_instance_context_mappings",
    "terminal_disposition",
    "unresolved_reason",
}
REVIEW_DOMAIN_ASSESSMENT_KEYS: Final = {
    "applicability",
    "evidence_refs",
    "review_domain",
}

JsonObject: TypeAlias = dict[str, object]


class ReviewWorklistError(ValueError):
    """A fail-closed worklist input or generation error."""

    def __init__(self, code: str, message: str, status: str = "FAIL") -> None:
        self.status = status
        self.code = code
        self.message = message
        super().__init__(f"[{status}:{code}] {message}")


@dataclass(frozen=True)
class LoadedReviewInputs:
    """Verified source projections used by the worklist and proposal tools."""

    source_commit: str
    model_binding: JsonObject
    candidate_universe_binding: JsonObject
    current_c_closure_binding: JsonObject
    classification_root_binding: JsonObject
    semantic_classes_binding: JsonObject
    classification_shard_bindings: tuple[JsonObject, ...]
    reviewer_roster_ref: JsonObject
    model: Mapping[str, object]
    candidate_universe: Mapping[str, object]
    current_c_closure: Mapping[str, object]
    classification_root: Mapping[str, object]
    semantic_classes: Mapping[str, object]
    classification_shards: tuple[Mapping[str, object], ...]
    candidate_records: tuple[Mapping[str, object], ...]
    source_instance_records: tuple[Mapping[str, object], ...]
    classification_records: tuple[Mapping[str, object], ...]
    review_domains: tuple[str, ...]
    unresolved_reasons: tuple[str, ...]


def _fail(code: str, message: str) -> NoReturn:
    raise ReviewWorklistError(code, message)


def _object(value: object, label: str) -> Mapping[str, object]:
    if not isinstance(value, Mapping):
        _fail("SOURCE_SHAPE_INVALID", f"{label} must be an object")
    return cast(Mapping[str, object], value)


def _array(value: object, label: str) -> list[object]:
    if not isinstance(value, list):
        _fail("SOURCE_SHAPE_INVALID", f"{label} must be an array")
    return cast(list[object], value)


def _text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        _fail("SOURCE_VALUE_INVALID", f"{label} must be non-empty text")
    return value


def _digest(value: object, label: str) -> str:
    value_text = _text(value, label)
    if len(value_text) != 64 or any(char not in "0123456789abcdef" for char in value_text):
        _fail("SOURCE_VALUE_INVALID", f"{label} must be lowercase SHA-256 hex")
    return value_text


def _exact_keys(value: Mapping[str, object], expected: set[str], label: str) -> None:
    if set(value) != expected:
        _fail("SOURCE_SHAPE_INVALID", f"{label} fields differ from the V1 source contract")


def _plain(value: object) -> object:
    if isinstance(value, Mapping):
        return {str(key): _plain(child) for key, child in value.items()}
    if isinstance(value, tuple | list):
        return [_plain(child) for child in value]
    return value


def _plain_object(value: Mapping[str, object]) -> JsonObject:
    return cast(JsonObject, _plain(value))


def _json_bytes(value: object) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode("utf-8")


def _git_head(repo_root: Path) -> str:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        _fail("SOURCE_COMMIT_UNAVAILABLE", f"cannot resolve repository HEAD: {exc}")
    head = result.stdout.strip()
    if len(head) != 40 or any(char not in "0123456789abcdef" for char in head):
        _fail("SOURCE_COMMIT_INVALID", "repository HEAD is not a lowercase Git SHA")
    return head


def _ensure_source_paths_clean(repo_root: Path, paths: Sequence[str]) -> None:
    for path in paths:
        for cached in (False, True):
            command = ["git", "diff"]
            if cached:
                command.append("--cached")
            command.extend(["--quiet", "HEAD", "--", path])
            try:
                result = subprocess.run(command, cwd=repo_root, check=False)
            except OSError as exc:
                _fail("SOURCE_STATUS_UNAVAILABLE", f"cannot inspect {path}: {exc}")
            if result.returncode != 0:
                _fail("SOURCE_TREE_DIRTY", f"bound source path is not committed: {path}")


def _read_json(
    resolver: AuthoritySourceResolver,
    path: str,
    expected_sha256: str,
    schema: str,
    label: str,
) -> Mapping[str, object]:
    try:
        artifact = resolver.resolve_repository_artifact(
            path, bytes.fromhex(expected_sha256), schema
        )
    except ResolutionError as exc:
        raise ReviewWorklistError(exc.code, f"{label}: {exc.message}", exc.status.value) from exc
    return _object(artifact.json_value, label)


def _local_json(
    repo_root: Path,
    resolver: AuthoritySourceResolver,
    path: str,
    schema: str,
    label: str,
    accepted_sha256: str | None = None,
) -> tuple[str, Mapping[str, object]]:
    source = repo_root / Path(*path.split("/"))
    try:
        raw = source.read_bytes()
    except OSError as exc:
        _fail("SOURCE_READ_FAILED", f"{label}: {exc}")
    actual_sha256 = hashlib.sha256(raw).hexdigest()
    if accepted_sha256 is not None and actual_sha256 != accepted_sha256:
        _fail("SOURCE_BINDING_MISMATCH", f"{label} is not the accepted source snapshot")
    return actual_sha256, _read_json(resolver, path, actual_sha256, schema, label)


def _closure_bindings(closure: Mapping[str, object]) -> dict[str, JsonObject]:
    bound: dict[str, JsonObject] = {}
    for index, item in enumerate(_array(closure.get("bound_semantic_inputs"), "C closure inputs")):
        record = _object(item, f"C closure inputs[{index}]")
        _exact_keys(record, {"path", "raw_sha256", "record_count", "schema"}, "C closure input")
        path = _text(record.get("path"), "C closure input path")
        if path in bound:
            _fail("DUPLICATE_SOURCE_BINDING", f"C closure binds {path!r} more than once")
        bound[path] = _plain_object(record)
    return bound


def _binding(path: str, schema: str, raw_sha256: str) -> JsonObject:
    return {"path": path, "schema": schema, "raw_sha256": raw_sha256}


def _production_roster(
    repo_root: Path, resolver: AuthoritySourceResolver
) -> tuple[JsonObject, str]:
    directory = repo_root / Path(*ROSTER_DIRECTORY.split("/"))
    try:
        files = sorted(directory.glob("*.json"))
    except OSError as exc:
        _fail("REVIEWER_ROSTER_UNAVAILABLE", f"cannot enumerate production roster: {exc}")
    if len(files) != 1 or files[0].name != Path(PRODUCTION_ROSTER_PATH).name:
        _fail(
            "REVIEWER_ROSTER_AMBIGUOUS",
            "production reviewer roster is not the accepted V1 leaf",
        )
    digest_hex = PRODUCTION_ROSTER_DIGEST
    path = PRODUCTION_ROSTER_PATH
    try:
        raw = files[0].read_bytes()
    except OSError as exc:
        _fail("REVIEWER_ROSTER_READ_FAILED", str(exc))
    actual_digest = hashlib.sha256(raw).hexdigest()
    if actual_digest != digest_hex:
        _fail(
            "REVIEWER_ROSTER_DIGEST_MISMATCH", "production roster basename differs from its bytes"
        )
    reference = ReviewerRosterRefV1(
        path=path,
        schema=REVIEWER_ROSTER_SCHEMA_V1,
        raw_sha256=bytes.fromhex(digest_hex),
    )
    try:
        resolver.resolve_reviewer_roster_leaf(reference)
    except ResolutionError as exc:
        raise ReviewWorklistError(exc.code, exc.message, exc.status.value) from exc
    return {
        "path": reference.path,
        "schema": reference.schema,
        "raw_sha256": digest_hex,
    }, path


def _load_inputs(repo_root: Path) -> LoadedReviewInputs:
    resolver = AuthoritySourceResolver(repo_root)
    source_commit = _git_head(repo_root)
    closure_sha, closure = _local_json(
        repo_root,
        resolver,
        CLOSURE_PATH,
        CLOSURE_SCHEMA,
        "C closure",
        ACCEPTED_CLOSURE_SHA256,
    )
    if closure.get("schema") != CLOSURE_SCHEMA:
        _fail("CLOSURE_SCHEMA_MISMATCH", "C closure schema is not V3")
    if closure.get("model_id") != "declared-interaction-model.v2":
        _fail("MODEL_BINDING_MISMATCH", "C closure model ID is not the accepted model")
    bindings = _closure_bindings(closure)
    required_paths = (
        MODEL_PATH,
        CANDIDATE_UNIVERSE_PATH,
        SEMANTIC_CLASSES_PATH,
        CLASSIFICATION_ROOT_PATH,
    )
    for path in required_paths:
        if path not in bindings:
            _fail("SOURCE_BINDING_MISSING", f"C closure does not bind {path}")
    source_paths = [CLOSURE_PATH, *required_paths]
    classification_root_sha = _digest(
        bindings[CLASSIFICATION_ROOT_PATH]["raw_sha256"], "classification root digest"
    )
    candidate_sha = _digest(
        bindings[CANDIDATE_UNIVERSE_PATH]["raw_sha256"], "candidate universe digest"
    )
    model_sha = _digest(bindings[MODEL_PATH]["raw_sha256"], "model digest")
    semantic_classes_sha = _digest(
        bindings[SEMANTIC_CLASSES_PATH]["raw_sha256"], "semantic classes digest"
    )
    model_schema = _text(bindings[MODEL_PATH]["schema"], "model schema")
    candidate_schema = _text(bindings[CANDIDATE_UNIVERSE_PATH]["schema"], "candidate schema")
    classes_schema = _text(bindings[SEMANTIC_CLASSES_PATH]["schema"], "semantic classes schema")
    classification_schema = _text(
        bindings[CLASSIFICATION_ROOT_PATH]["schema"], "classification root schema"
    )
    if model_schema != DECLARED_MODEL_SCHEMA:
        _fail("SOURCE_BINDING_MISMATCH", "C closure model schema is not V2")
    if candidate_schema != CANDIDATE_UNIVERSE_SCHEMA:
        _fail("SOURCE_BINDING_MISMATCH", "C closure candidate schema is not V2")
    if classes_schema != SEMANTIC_CLASSES_SCHEMA:
        _fail("SOURCE_BINDING_MISMATCH", "C closure semantic classes schema is not V2")
    if classification_schema != CLASSIFICATION_ROOT_SCHEMA:
        _fail("SOURCE_BINDING_MISMATCH", "C closure classification schema is not V3")
    roster_ref, roster_path = _production_roster(repo_root, resolver)
    source_paths.append(roster_path)
    _ensure_source_paths_clean(repo_root, source_paths)
    model = _read_json(resolver, MODEL_PATH, model_sha, model_schema, "declared model")
    candidate_binding = SourceBindingDigestV1(
        "candidate_universe",
        CANDIDATE_UNIVERSE_PATH,
        candidate_schema,
        bytes.fromhex(candidate_sha),
    )
    try:
        candidate_index = resolver._candidate_universe(candidate_binding)
    except ResolutionError as exc:
        raise ReviewWorklistError(exc.code, exc.message, exc.status.value) from exc
    candidate_universe = _object(candidate_index.artifact.json_value, "candidate universe")
    semantic_classes = _read_json(
        resolver, SEMANTIC_CLASSES_PATH, semantic_classes_sha, classes_schema, "semantic classes"
    )
    classification_root = _read_json(
        resolver,
        CLASSIFICATION_ROOT_PATH,
        classification_root_sha,
        classification_schema,
        "classification root",
    )
    if semantic_classes.get("class_count") != 0:
        _fail("UNEXPECTED_SEMANTIC_CLASSES", "current semantic class source is not empty")
    root_shards = _array(classification_root.get("shards"), "classification root shards")
    classification_shards: list[Mapping[str, object]] = []
    shard_bindings: list[JsonObject] = []
    for index, item in enumerate(root_shards):
        descriptor = _object(item, f"classification root shard[{index}]")
        _exact_keys(
            descriptor,
            {
                "shard_index",
                "path",
                "ordinal_start",
                "ordinal_end_exclusive",
                "record_count",
                "first_candidate_id",
                "last_candidate_id",
                "raw_sha256",
            },
            "classification shard descriptor",
        )
        relative_shard = _text(descriptor.get("path"), "classification shard path")
        shard_path = f"{C_DIR}/{relative_shard}"
        shard_sha = _digest(descriptor.get("raw_sha256"), "classification shard digest")
        classification_shards.append(
            _read_json(
                resolver,
                shard_path,
                shard_sha,
                CLASSIFICATION_SHARD_SCHEMA,
                f"classification shard[{index}]",
            )
        )
        shard_bindings.append(_binding(shard_path, CLASSIFICATION_SHARD_SCHEMA, shard_sha))
        source_paths.append(shard_path)
    _ensure_source_paths_clean(repo_root, source_paths)
    source_instance_records = tuple(
        _plain_object(cast(Mapping[str, object], item))
        for item in candidate_index.instances_by_id.values()
    )
    candidate_records = tuple(
        _plain_object(cast(Mapping[str, object], item))
        for item in candidate_index.candidates_by_id.values()
    )
    classification_records: list[Mapping[str, object]] = []
    for shard in classification_shards:
        classification_records.extend(
            _plain_object(cast(Mapping[str, object], item))
            for item in _array(shard.get("candidate_classifications"), "classification records")
        )
    review_domains = tuple(
        _text(item, "review domain")
        for item in _array(model.get("review_domain_vocabulary"), "review domains")
    )
    unresolved_reasons = tuple(
        _text(item, "unresolved reason")
        for item in _array(model.get("unresolved_reasons"), "unresolved reasons")
    )
    return LoadedReviewInputs(
        source_commit=source_commit,
        model_binding=_binding(MODEL_PATH, model_schema, model_sha),
        candidate_universe_binding=_binding(
            CANDIDATE_UNIVERSE_PATH, candidate_schema, candidate_sha
        ),
        current_c_closure_binding=_binding(CLOSURE_PATH, CLOSURE_SCHEMA, closure_sha),
        classification_root_binding=_binding(
            CLASSIFICATION_ROOT_PATH, classification_schema, classification_root_sha
        ),
        semantic_classes_binding=_binding(
            SEMANTIC_CLASSES_PATH, classes_schema, semantic_classes_sha
        ),
        classification_shard_bindings=tuple(shard_bindings),
        reviewer_roster_ref=roster_ref,
        model=_plain_object(model),
        candidate_universe=_plain_object(candidate_universe),
        current_c_closure=_plain_object(closure),
        classification_root=_plain_object(classification_root),
        semantic_classes=_plain_object(semantic_classes),
        classification_shards=tuple(_plain_object(shard) for shard in classification_shards),
        candidate_records=candidate_records,
        source_instance_records=source_instance_records,
        classification_records=tuple(classification_records),
        review_domains=review_domains,
        unresolved_reasons=unresolved_reasons,
    )


def _validate_classification(
    record: Mapping[str, object],
    index: int,
    candidate_ids: set[str],
    review_domains: tuple[str, ...],
    unresolved_reasons: set[str],
) -> str:
    _exact_keys(record, CLASSIFICATION_RECORD_KEYS, "classification record")
    candidate_id = _text(record.get("candidate_id"), f"classification[{index}].candidate_id")
    if candidate_id not in candidate_ids:
        _fail("CLASSIFICATION_CANDIDATE_MISMATCH", f"classification references {candidate_id!r}")
    if record.get("review_state") != REVIEW_STATE_UNRESOLVED:
        _fail("UNEXPECTED_REVIEW_STATE", f"classification {candidate_id!r} is not unresolved")
    if (
        record.get("terminal_disposition") is not None
        or record.get("interaction_class_id") is not None
    ):
        _fail("UNEXPECTED_TERMINAL_FIELDS", f"classification {candidate_id!r} has terminal fields")
    reason = _text(record.get("unresolved_reason"), f"classification[{index}].unresolved_reason")
    if reason not in unresolved_reasons:
        _fail("UNRESOLVED_REASON_UNKNOWN", f"classification {candidate_id!r} has unknown reason")
    assessments = _array(
        record.get("review_domain_assessments"),
        f"classification[{index}].review_domain_assessments",
    )
    if len(assessments) != len(review_domains):
        _fail(
            "REVIEW_DOMAIN_COVERAGE_MISMATCH",
            f"classification {candidate_id!r} has incomplete domains",
        )
    seen: set[str] = set()
    for domain_index, item in enumerate(assessments):
        assessment = _object(
            item, f"classification[{index}].review_domain_assessments[{domain_index}]"
        )
        _exact_keys(assessment, REVIEW_DOMAIN_ASSESSMENT_KEYS, "review-domain assessment")
        domain = _text(assessment.get("review_domain"), "review domain")
        if domain not in review_domains or domain in seen or domain != review_domains[domain_index]:
            _fail(
                "UNKNOWN_REVIEW_DOMAIN",
                f"classification {candidate_id!r} has noncanonical domain order",
            )
        seen.add(domain)
        if assessment.get("applicability") != "unresolved":
            _fail(
                "UNEXPECTED_DOMAIN_STATE", f"classification {candidate_id!r} has a resolved domain"
            )
    return candidate_id


def validate_review_inputs(inputs: LoadedReviewInputs) -> None:
    """Validate only the closed structural state needed by worklist output."""

    if len(inputs.source_commit) != 40 or any(
        char not in "0123456789abcdef" for char in inputs.source_commit
    ):
        _fail("SOURCE_COMMIT_INVALID", "source_commit is not a lowercase Git SHA")
    expected_bindings = {
        "model_binding": _binding(MODEL_PATH, DECLARED_MODEL_SCHEMA, ACCEPTED_MODEL_SHA256),
        "candidate_universe_binding": _binding(
            CANDIDATE_UNIVERSE_PATH,
            CANDIDATE_UNIVERSE_SCHEMA,
            ACCEPTED_CANDIDATE_UNIVERSE_SHA256,
        ),
        "current_c_closure_binding": _binding(
            CLOSURE_PATH, CLOSURE_SCHEMA, ACCEPTED_CLOSURE_SHA256
        ),
        "classification_root_binding": _binding(
            CLASSIFICATION_ROOT_PATH,
            CLASSIFICATION_ROOT_SCHEMA,
            ACCEPTED_CLASSIFICATION_ROOT_SHA256,
        ),
        "semantic_classes_binding": _binding(
            SEMANTIC_CLASSES_PATH,
            SEMANTIC_CLASSES_SCHEMA,
            ACCEPTED_SEMANTIC_CLASSES_SHA256,
        ),
    }
    for name, expected in expected_bindings.items():
        if getattr(inputs, name) != expected:
            _fail("SOURCE_BINDING_MISMATCH", f"{name} is not the accepted source snapshot")
    if inputs.model.get("schema") != DECLARED_MODEL_SCHEMA:
        _fail("MODEL_BINDING_MISMATCH", "declared model schema is not V2")
    if inputs.model.get("model_id") != "declared-interaction-model.v2":
        _fail("MODEL_BINDING_MISMATCH", "declared model ID is not the accepted C model")
    expected_roster = _binding(
        PRODUCTION_ROSTER_PATH, REVIEWER_ROSTER_SCHEMA_V1, PRODUCTION_ROSTER_DIGEST
    )
    if inputs.reviewer_roster_ref != expected_roster:
        _fail("SOURCE_BINDING_MISMATCH", "reviewer roster is not the accepted production leaf")
    if not inputs.review_domains or len(set(inputs.review_domains)) != len(inputs.review_domains):
        _fail(
            "REVIEW_DOMAIN_COVERAGE_MISMATCH",
            "review domain vocabulary is not a unique ordered list",
        )
    candidate_ids: list[str] = []
    candidate_by_id: dict[str, Mapping[str, object]] = {}
    for index, candidate in enumerate(inputs.candidate_records):
        candidate_id = _text(candidate.get("candidate_id"), f"candidate[{index}].candidate_id")
        if candidate_id in candidate_by_id:
            _fail("DUPLICATE_CANDIDATE_ID", f"candidate {candidate_id!r} appears more than once")
        candidate_ids.append(candidate_id)
        candidate_by_id[candidate_id] = candidate
        _object(candidate.get("candidate_identity"), f"candidate[{index}].candidate_identity")
        _object(candidate.get("source_binding"), f"candidate[{index}].source_binding")
    if inputs.candidate_universe.get("candidate_count") != len(candidate_ids):
        _fail("CANDIDATE_CARDINALITY_MISMATCH", "candidate count does not match candidate records")
    if inputs.candidate_universe.get("source_instance_count") != len(
        inputs.source_instance_records
    ):
        _fail(
            "SOURCE_INSTANCE_CARDINALITY_MISMATCH", "source-instance count does not match records"
        )
    instance_by_candidate: dict[str, Mapping[str, object]] = {}
    instance_ids: set[str] = set()
    for index, instance in enumerate(inputs.source_instance_records):
        instance_id = _text(
            instance.get("source_instance_id"), f"source_instance[{index}].source_instance_id"
        )
        if instance_id in instance_ids:
            _fail(
                "DUPLICATE_SOURCE_INSTANCE_ID",
                f"source instance {instance_id!r} appears more than once",
            )
        instance_ids.add(instance_id)
        candidate_id = _text(instance.get("candidate_id"), f"source_instance[{index}].candidate_id")
        candidate = candidate_by_id.get(candidate_id)
        if candidate is None:
            _fail(
                "SOURCE_INSTANCE_CANDIDATE_MISMATCH",
                f"source instance {instance_id!r} has no candidate",
            )
        if instance.get("source_binding") != candidate.get("source_binding"):
            _fail(
                "CANDIDATE_SOURCE_BINDING_MISMATCH",
                f"source instance {instance_id!r} binding differs",
            )
        if candidate_id in instance_by_candidate:
            _fail(
                "SOURCE_INSTANCE_COVERAGE_MISMATCH",
                f"candidate {candidate_id!r} has multiple source instances",
            )
        instance_by_candidate[candidate_id] = instance
    if set(instance_by_candidate) != set(candidate_by_id):
        _fail(
            "SOURCE_INSTANCE_COVERAGE_MISMATCH", "candidate/source-instance coverage is incomplete"
        )
    _exact_keys(inputs.classification_root, CLASSIFICATION_ROOT_KEYS, "classification root")
    if inputs.classification_root.get("model_id") != inputs.model.get("model_id"):
        _fail("MODEL_BINDING_MISMATCH", "classification root model differs from declared model")
    if (
        inputs.classification_root.get("candidate_universe_raw_sha256")
        != inputs.candidate_universe_binding["raw_sha256"]
    ):
        _fail(
            "SOURCE_BINDING_MISMATCH",
            "classification root candidate digest differs from candidate universe",
        )
    if (
        inputs.classification_root.get("semantic_classes_raw_sha256")
        != inputs.semantic_classes_binding["raw_sha256"]
    ):
        _fail(
            "SOURCE_BINDING_MISMATCH",
            "classification root semantic-class digest differs from source",
        )
    if inputs.classification_root.get("classification_count") != len(inputs.classification_records):
        _fail("CLASSIFICATION_CARDINALITY_MISMATCH", "classification count does not match records")
    if inputs.classification_root.get("classification_count") != len(candidate_ids):
        _fail(
            "CLASSIFICATION_CARDINALITY_MISMATCH", "candidate/classification cardinalities differ"
        )
    if inputs.classification_root.get("partition_scheme") != PARTITION_SCHEME:
        _fail(
            "CLASSIFICATION_ORDER_INVALID",
            "classification partition scheme is not the accepted order",
        )
    if (
        inputs.semantic_classes.get("schema") != SEMANTIC_CLASSES_SCHEMA
        or inputs.semantic_classes.get("model_id") != inputs.model.get("model_id")
        or inputs.semantic_classes.get("class_count") != 0
        or inputs.semantic_classes.get("classes") != []
    ):
        _fail("SOURCE_BINDING_MISMATCH", "semantic-class source differs from the empty C baseline")
    root_shards = _array(inputs.classification_root.get("shards"), "classification root shards")
    if inputs.classification_root.get("shard_count") != len(root_shards):
        _fail("CLASSIFICATION_SHARD_COVERAGE_MISMATCH", "classification root shard count differs")
    if len(root_shards) != len(inputs.classification_shards) or len(root_shards) != len(
        inputs.classification_shard_bindings
    ):
        _fail(
            "CLASSIFICATION_SHARD_COVERAGE_MISMATCH", "classification shard coverage is incomplete"
        )
    flattened_ids: list[str] = []
    expected_start = 0
    for index, (raw_descriptor, shard, binding) in enumerate(
        zip(
            root_shards,
            inputs.classification_shards,
            inputs.classification_shard_bindings,
            strict=True,
        )
    ):
        descriptor = _object(raw_descriptor, f"classification shard descriptor[{index}]")
        shard_index = descriptor.get("shard_index")
        if shard_index != index or descriptor.get("ordinal_start") != expected_start:
            _fail(
                "CLASSIFICATION_ORDER_INVALID",
                f"classification shard[{index}] has a noncanonical range",
            )
        records = _array(
            shard.get("candidate_classifications"), f"classification shard[{index}] records"
        )
        _exact_keys(shard, CLASSIFICATION_SHARD_KEYS, "classification shard")
        end = descriptor.get("ordinal_end_exclusive")
        if not isinstance(end, int) or end != expected_start + len(records):
            _fail(
                "CLASSIFICATION_SHARD_COVERAGE_MISMATCH",
                f"classification shard[{index}] count/range differs",
            )
        if descriptor.get("record_count") != len(records):
            _fail(
                "CLASSIFICATION_SHARD_COVERAGE_MISMATCH",
                f"classification shard[{index}] record count differs",
            )
        if binding.get("raw_sha256") != descriptor.get("raw_sha256"):
            _fail("SOURCE_BINDING_MISMATCH", f"classification shard[{index}] digest differs")
        expected_binding = _binding(
            f"{C_DIR}/{_text(descriptor.get('path'), 'classification shard path')}",
            CLASSIFICATION_SHARD_SCHEMA,
            _digest(descriptor.get("raw_sha256"), "classification shard digest"),
        )
        if binding != expected_binding:
            _fail("SOURCE_BINDING_MISMATCH", f"classification shard[{index}] binding differs")
        if (
            shard.get("schema") != CLASSIFICATION_SHARD_SCHEMA
            or shard.get("model_id") != inputs.model.get("model_id")
            or shard.get("candidate_universe_raw_sha256")
            != inputs.candidate_universe_binding["raw_sha256"]
            or shard.get("semantic_classes_raw_sha256")
            != inputs.semantic_classes_binding["raw_sha256"]
            or shard.get("partition_scheme") != PARTITION_SCHEME
            or shard.get("shard_count") != len(root_shards)
            or shard.get("shard_index") != index
        ):
            _fail("SOURCE_BINDING_MISMATCH", f"classification shard[{index}] header differs")
        shard_ids = [
            _text(
                _object(record, "classification record").get("candidate_id"),
                "classification candidate ID",
            )
            for record in records
        ]
        if records and (
            descriptor.get("first_candidate_id") != shard_ids[0]
            or descriptor.get("last_candidate_id") != shard_ids[-1]
        ):
            _fail(
                "CLASSIFICATION_ORDER_INVALID", f"classification shard[{index}] boundary IDs differ"
            )
        flattened_ids.extend(shard_ids)
        expected_start = end
    classification_ids: list[str] = []
    classification_id_set: set[str] = set()
    for index, record in enumerate(inputs.classification_records):
        candidate_id = _text(record.get("candidate_id"), f"classification[{index}].candidate_id")
        if candidate_id in classification_id_set:
            _fail(
                "DUPLICATE_CLASSIFICATION_CANDIDATE",
                f"classification candidate {candidate_id!r} appears more than once",
            )
        classification_id_set.add(candidate_id)
        classification_ids.append(candidate_id)
    if classification_ids != flattened_ids:
        _fail(
            "CLASSIFICATION_SHARD_SOURCE_MISMATCH",
            "classification records do not match the verified shard records",
        )
    if classification_id_set != set(candidate_ids):
        _fail("CLASSIFICATION_CANDIDATE_MISMATCH", "classification and candidate sets differ")
    for index, record in enumerate(inputs.classification_records):
        _validate_classification(
            record, index, set(candidate_ids), inputs.review_domains, set(inputs.unresolved_reasons)
        )
    metrics = _object(
        inputs.current_c_closure.get("review_state_metrics"), "C closure review-state metrics"
    )
    if metrics != {
        "resolved_not_an_interaction_with_proof": 0,
        "resolved_out_of_declared_scope_with_reason": 0,
        "resolved_required_interaction": 0,
        "unresolved": len(candidate_ids),
    }:
        _fail("C_STATE_UNEXPECTED", "current C closure metrics are not the unresolved baseline")


def _review_obligations(review_domains: Sequence[str]) -> JsonObject:
    return {
        "relation_review": {"status": "awaiting_human_semantic_review"},
        "domain_reviews": [
            {"review_domain": domain, "status": "awaiting_human_semantic_review"}
            for domain in review_domains
        ],
        "conditional_context_review": "defer_until_accepted_semantic_result",
        "conditional_scope_review": "defer_until_accepted_semantic_result",
    }


def _work_item(
    ordinal: int,
    candidate: Mapping[str, object],
    source_instance: Mapping[str, object],
    classification: Mapping[str, object],
    review_domains: Sequence[str],
) -> JsonObject:
    return {
        "record_type": "review_work_item",
        "ordinal": ordinal,
        "candidate_id": candidate["candidate_id"],
        "candidate_identity": _plain(candidate["candidate_identity"]),
        "source_instance_id": source_instance["source_instance_id"],
        "scope": candidate["scope"],
        "relation": candidate["relation"],
        "participant_refs": _plain(candidate["participant_refs"]),
        "candidate_source_binding": _plain(candidate["source_binding"]),
        "source_instance_binding": _plain(source_instance["source_binding"]),
        "current_review_state": classification["review_state"],
        "current_unresolved_reason": classification["unresolved_reason"],
        "review_obligations": _review_obligations(review_domains),
    }


def _manifest(inputs: LoadedReviewInputs) -> JsonObject:
    return {
        "record_type": "review_worklist_manifest",
        "format": WORKLIST_FORMAT,
        "source_commit": inputs.source_commit,
        "declared_model": _plain(inputs.model_binding),
        "candidate_universe": _plain(inputs.candidate_universe_binding),
        "current_c_closure": _plain(inputs.current_c_closure_binding),
        "classification_root": _plain(inputs.classification_root_binding),
        "semantic_classes": _plain(inputs.semantic_classes_binding),
        "classification_shards": _plain(inputs.classification_shard_bindings),
        "reviewer_roster": _plain(inputs.reviewer_roster_ref),
        "candidate_count": len(inputs.candidate_records),
        "work_item_count": len(inputs.candidate_records),
        "classification_count": len(inputs.classification_records),
        "classification_state": {"resolved": 0, "unresolved": len(inputs.classification_records)},
    }


def build_worklist(
    repo_root: Path = ROOT,
    output_dir: Path | None = None,
    *,
    inputs: LoadedReviewInputs | None = None,
) -> Path:
    """Write a deterministic JSONL worklist and deterministic summary."""

    loaded = inputs or _load_inputs(repo_root)
    validate_review_inputs(loaded)
    candidate_by_id = {
        _text(candidate.get("candidate_id"), "candidate ID"): candidate
        for candidate in loaded.candidate_records
    }
    instance_by_candidate = {
        _text(instance.get("candidate_id"), "source-instance candidate ID"): instance
        for instance in loaded.source_instance_records
    }
    classifications_by_id = {
        _text(record.get("candidate_id"), "classification candidate ID"): record
        for record in loaded.classification_records
    }
    lines = [_json_bytes(_manifest(loaded))]
    for ordinal, classification in enumerate(loaded.classification_records):
        candidate_id = _text(classification.get("candidate_id"), "classification candidate ID")
        lines.append(
            _json_bytes(
                _work_item(
                    ordinal,
                    candidate_by_id[candidate_id],
                    instance_by_candidate[candidate_id],
                    classifications_by_id[candidate_id],
                    loaded.review_domains,
                )
            )
        )
    worklist_raw = b"".join(lines)
    target_dir = output_dir or repo_root / "dist" / "m2-5-c-authority-review"
    target_dir.mkdir(parents=True, exist_ok=True)
    worklist_path = target_dir / WORKLIST_NAME
    worklist_path.write_bytes(worklist_raw)
    worklist_digest = hashlib.sha256(worklist_raw).hexdigest()
    summary = (
        "# M2.5.C Authority Review Worklist\n\n"
        "This generated worklist is review input only, not authority, acceptance, "
        "or C classification.\n\n"
        f"- format: {WORKLIST_FORMAT}\n"
        f"- source commit: {loaded.source_commit}\n"
        f"- candidate/work-item count: {len(loaded.candidate_records)}\n"
        f"- unresolved count: {len(loaded.classification_records)}\n"
        f"- worklist SHA-256: {worklist_digest}\n"
        "- semantic grouping: none; one work item per existing candidate-order entry\n"
        "- semantic conclusions: none emitted\n"
    ).encode()
    (target_dir / SUMMARY_NAME).write_bytes(summary)
    return worklist_path


def load_review_inputs(repo_root: Path = ROOT) -> LoadedReviewInputs:
    """Load and validate the accepted source snapshot without generating output."""

    inputs = _load_inputs(repo_root)
    validate_review_inputs(inputs)
    return inputs


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=None,
        help="test/output override; defaults to dist/m2-5-c-authority-review",
    )
    args = parser.parse_args()
    try:
        path = build_worklist(ROOT, args.output_dir)
    except ReviewWorklistError as exc:
        print(f"{exc.status}: {exc.code}: {exc.message}", file=sys.stderr)
        return 2 if exc.status == "BLOCKED" else 1
    raw = path.read_bytes()
    print(f"PASS: generated {path.relative_to(ROOT)} sha256={hashlib.sha256(raw).hexdigest()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
