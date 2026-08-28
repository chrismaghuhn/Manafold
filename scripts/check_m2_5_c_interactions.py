#!/usr/bin/env python3
"""Fail-closed verifier and deterministic builder for the M2.5.C V3 snapshot.

The C snapshot has two executable trust layers.  The historical REV3/B1/B2
inputs are validated first, then the five C semantic inputs are checked as an
acyclic source graph (model -> review additions -> candidate universe ->
semantic classes -> classifications -> closure).  Candidate classifications
carry the unchanged V2 review-state grammar: unresolved is representable and
fail-closed, but is never a terminal disposition and never permits C acceptance.
The V3 classification root and its bounded shards are a layout-only container
for those unchanged records.  The report,
negative matrix, and verification summary are evidence projections and are
never closure inputs.  No keyword, capability-name, score, or language-native
serialization is used as semantic authority.
"""

from __future__ import annotations

import argparse
import base64
import copy
import csv
import hashlib
import io
import json
import os
import re
import struct
import subprocess
import sys
import tempfile
import zipfile
from collections import Counter
from collections.abc import Callable
from pathlib import Path
from typing import Any, NoReturn, cast

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
C_DIR = ROOT / "sources" / "m2_5" / "closures" / "C"
VERIFICATION_DIR = C_DIR / "verification"
ARCHIVE_ENV_VAR = "MANAFOLD_SOURCE_ARCHIVE"
ARCHIVE_RELATIVE_PATH = Path("m2_5/Manafold_M2_5_Pre_Research_ALL_ARTIFACTS_REV3.zip")
EXPECTED_ARCHIVE_SHA256 = "99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90"
EXPECTED_REV3_MODEL_ID = "interaction-model.v1"
EXPECTED_REV3_MODEL_MEMBER = "inputs/interaction_model_v1.json"
EXPECTED_REV3_MODEL_SHA256 = "f7a069df5040e9337719aadf0c1c4bde09a4b5dad0bb6489eada49d369a9bc8f"
EXPECTED_REV3_CENSUS_MEMBER = "derived/Pair_Interaction_Census_REV3.csv"
EXPECTED_REV3_CENSUS_SHA256 = "82f9312113bb1007ad6562d454c515f85dbc1e0d7a471f7b1c6793725aea45d4"
EXPECTED_REV3_CANDIDATE_COUNT = 15679
EXPECTED_RESOLUTION_MEMBER = "inputs/deck_row_source_resolution_REV3.csv"
EXPECTED_INDEX_MEMBER = "source/raw/source_record_index_REV3.csv"
EXPECTED_PREVIOUS_MASTER = "186d4b69ee406b19e1707d3f067f2bec14af3a34"
EXPECTED_SPEC_SHA256 = "79ca999c5e93b4293133aee9c8c5f7b6e404e7a5b9326154dcc1ba52011c1737"

MODEL_NAME = "declared_interaction_model.v2.json"
REVIEW_NAME = "interaction_review_additions.v2.json"
UNIVERSE_NAME = "interaction_candidate_universe.v2.json"
CLASSES_NAME = "interaction_semantic_classes.v2.json"
CLASSIFICATIONS_NAME = "interaction_classifications.v3.json"
SHARD_SCHEMA_NAME = "classification_shard"
CLOSURE_NAME = "interaction_closure.v3.json"
REPORT_NAME = "INTERACTION_MODEL_REPORT.md"
MATRIX_NAME = "c_negative_test_matrix.v3.json"
SUMMARY_NAME = "c_verification_summary.v3.json"
CHECKER_RELATIVE_PATH = "scripts/check_m2_5_c_interactions.py"
SHARD_DIRECTORY = "classification_shards"
SHARD_COUNT = 16
PARTITION_SCHEME = "candidate-order-fixed-chunk-1000-v1"
SHARD_MAX_BYTES = 50 * 1024 * 1024
HOSTING_HARD_LIMIT_BYTES = 100 * 1024 * 1024
MIGRATION_SOURCE_EXECUTION_COMMIT = "43bf3ccc6ff639c900914947ee0883b4731b8409"
MIGRATION_SOURCE_CLASSIFICATIONS_PATH = (
    "sources/m2_5/closures/C/interaction_classifications.v2.json"
)
SHARD_NAMES = tuple(
    f"{SHARD_DIRECTORY}/interaction_classifications.{index:04d}.v3.json"
    for index in range(SHARD_COUNT)
)

C_ARTIFACT_RELATIVE_PATHS = (
    "sources/m2_5/closures/C/C_DESIGN_SPEC.md",
    "sources/m2_5/closures/C/declared_interaction_model.v2.json",
    "sources/m2_5/closures/C/interaction_review_additions.v2.json",
    "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
    "sources/m2_5/closures/C/interaction_semantic_classes.v2.json",
    "sources/m2_5/closures/C/interaction_classifications.v3.json",
    *tuple(f"sources/m2_5/closures/C/{name}" for name in SHARD_NAMES),
    "sources/m2_5/closures/C/interaction_closure.v3.json",
    "sources/m2_5/closures/C/INTERACTION_MODEL_REPORT.md",
    "sources/m2_5/closures/C/verification/c_negative_test_matrix.v3.json",
    "sources/m2_5/closures/C/verification/c_verification_summary.v3.json",
)
EXPECTED_C_DIRECTORY_FILES = frozenset(
    {
        "C_DESIGN_SPEC.md",
        MODEL_NAME,
        REVIEW_NAME,
        UNIVERSE_NAME,
        CLASSES_NAME,
        CLASSIFICATIONS_NAME,
        *SHARD_NAMES,
        CLOSURE_NAME,
        REPORT_NAME,
        f"verification/{MATRIX_NAME}",
        f"verification/{SUMMARY_NAME}",
    }
)
C_JSON_SCHEMAS = {
    MODEL_NAME: "manafold.m2.5.c.declared-interaction-model.v2",
    REVIEW_NAME: "manafold.m2.5.c.interaction-review-additions.v2",
    UNIVERSE_NAME: "manafold.m2.5.c.interaction-candidate-universe.v2",
    CLASSES_NAME: "manafold.m2.5.c.interaction-semantic-classes.v2",
    CLASSIFICATIONS_NAME: "manafold.m2.5.c.interaction-classifications.v3",
    SHARD_SCHEMA_NAME: "manafold.m2.5.c.interaction-classifications-shard.v3",
    CLOSURE_NAME: "manafold.m2.5.c.interaction-closure.v3",
    MATRIX_NAME: "manafold.m2.5.c.negative-test-matrix.v3",
    SUMMARY_NAME: "manafold.m2.5.c.verification-summary.v3",
}

EXPECTED_GATE_STATUS = {
    "CLASSIFICATION_REFERENCE_CLOSURE": "PASS",
    "OFFICIAL_RULE_CITATION_CLOSURE": "PASS",
    "DECLARED_INTERACTION_MODEL_CLOSURE": "BLOCKED",
    "REV2_REUSE_RATIO_REPRODUCIBLE": "BLOCKED",
    "RANKING_UNCERTAINTY_PROPAGATION": "BLOCKED",
}
EXPECTED_PASS_GATE_STATUS = {**EXPECTED_GATE_STATUS, "DECLARED_INTERACTION_MODEL_CLOSURE": "PASS"}
EXPECTED_FLAGS = {
    "DECK_PAIR_LOCKED": False,
    "AUTHORITATIVE_RANKING_AVAILABLE": False,
    "M3_STARTED": False,
}

PARTICIPANT_KINDS = (
    "ability",
    "card",
    "copiable_value",
    "deck",
    "effect",
    "event",
    "object",
    "permanent",
    "player",
    "requirement_family",
    "source_instance",
    "spell",
    "token",
    "zone",
)
PARTICIPANT_ROLES = (
    "affected",
    "controller",
    "copied_source",
    "copy_result",
    "decision_actor",
    "destination_zone",
    "origin_zone",
    "ordered_participant",
    "owner",
    "replacement_actor",
    "source",
    "target",
    "trigger_source",
)
CONTEXT_VOCABULARY = {
    "zone": (
        "battlefield",
        "command_zone",
        "exile",
        "graveyard",
        "hand",
        "library",
        "outside_game",
        "stack",
        "zone_agnostic",
        "not_applicable",
    ),
    "visibility": (
        "controller_only",
        "hidden_to_actor",
        "identity_hidden",
        "not_applicable",
        "owner_only",
        "private",
        "public",
    ),
    "timing": (
        "activation_time",
        "cast_time",
        "combat_time",
        "continuous_effect",
        "not_applicable",
        "resolution_time",
        "state_based_check",
        "trigger_time",
        "turn_boundary",
        "zone_change_time",
    ),
    "temporal_order": (
        "after",
        "before",
        "during",
        "not_applicable",
        "sequential",
        "simultaneous",
        "until",
        "while",
    ),
    "source_affected_relation": (
        "both_affected",
        "no_effect_relation",
        "not_applicable",
        "source_affected",
        "source_affects_other",
    ),
    "control_ownership_relation": (
        "control_changes",
        "cross_controller",
        "cross_owner",
        "not_applicable",
        "ownership_changes",
        "same_controller",
        "same_owner",
    ),
    "replacement_layer_relation": (
        "copy_layer",
        "control_layer",
        "layer_dependency",
        "no_replacement_or_layer",
        "not_applicable",
        "pt_layer",
        "replacement_effect",
        "type_layer",
        "zone_change_replacement",
    ),
    "trigger_lki_relation": (
        "intervening_if",
        "last_known_information",
        "no_trigger_lki",
        "not_applicable",
        "trigger_condition",
        "triggered_event",
    ),
    "information_relation": (
        "hidden_identity",
        "known_to_controller",
        "known_to_owner",
        "no_information_dependency",
        "not_applicable",
        "private_look",
        "public_identity",
        "random_unknown",
    ),
    "decision_actor_relation": (
        "active_player",
        "controller",
        "no_decision",
        "not_applicable",
        "opponent",
        "owner",
        "rules_forced",
        "target_player",
    ),
}
TEMPORAL_VOCABULARY = {
    "dependency_order": (
        "dependency_ordered",
        "no_temporal_dependency",
        "not_applicable",
    ),
    "duration": ("duration_limited", "indefinite", "not_applicable", "until_event"),
    "replacement_order": (
        "after_effect",
        "before_effect",
        "no_temporal_dependency",
        "not_applicable",
        "same_event",
    ),
    "trigger_order": ("deferred", "immediate", "no_temporal_dependency", "not_applicable"),
}
CONTEXT_KEYS = tuple(CONTEXT_VOCABULARY)
TEMPORAL_KEYS = tuple(TEMPORAL_VOCABULARY)
EXTRA_VOCABULARY = {
    "coverage_scope": ("pairwise_plus_review_outliers",),
    "arity": ("unary", "binary", "higher_order"),
    "directionality": ("directed", "none", "symmetric"),
    "host_relationship": ("cross_host", "not_applicable", "same_host"),
    "authority_kind": ("b1_final", "b2", "c_review", "rev3"),
    "assignment_role": ("primary", "supporting"),
    "lifecycle": ("active", "active_unassigned"),
    "source_origin": ("rev3", "targeted_higher_order_review"),
    "scope": ("cross_deck", "intra_deck", "unary_or_higher_order"),
    "relation": (
        "declared_card_trigger",
        "directional_binary",
        "reviewed_higher_order",
        "unordered_binary",
    ),
    "review_kind": ("targeted_higher_order_review",),
    "source_kind": ("b2_assignment", "b2_classification", "rev3_row"),
    "terminal_disposition": (
        "required_interaction",
        "not_an_interaction_with_proof",
        "out_of_declared_scope_with_reason",
    ),
    "reconciliation_status": (
        "unchanged",
        "stale_rev3_candidate",
        "removed_not_interaction",
        "merged_semantic_duplicate",
        "new_targeted_higher_order_candidate",
    ),
}

MODEL_ID = "declared-interaction-model.v2"
MODEL_VERSION = "2"
REVIEW_STATES = ("resolved", "unresolved")
UNRESOLVED_REASONS = (
    "insufficient_pair_relation_authority",
    "insufficient_boundary_relation_evidence",
    "missing_required_review_evidence",
)
REVIEW_DOMAINS = (
    "triggers_and_lki",
    "replacement_layers_and_dependency",
    "copy_and_token_creation",
    "target_legality_protection_and_identity",
    "control_and_ownership",
    "commander_and_format",
    "hidden_information_and_visibility",
    "ordering_and_temporal_dependencies",
    "source_versus_affected_identity",
    "controller_owner_and_decision_actor",
    "higher_order_interactions",
)
REVIEW_DOMAIN_APPLICABILITY = ("applicable", "not_applicable", "unresolved")

HEX64_RE = re.compile(r"^[0-9a-f]{64}$")
UUID_RE = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
REVIEW_ID_RE = re.compile(r"^ira\.v1/[a-z0-9][a-z0-9._-]*$")

encode_canonical: Any = None
encode_envelope: Any = None
_PERSISTENCE_IMPORT_ERROR: Exception | None = None
try:
    PYTHON_SRC = ROOT / "python" / "src"
    if str(PYTHON_SRC) not in sys.path:
        sys.path.insert(0, str(PYTHON_SRC))
    from mtgml.persistence import encode_canonical as _encode_canonical
    from mtgml.persistence import encode_envelope as _encode_envelope

    encode_canonical = _encode_canonical
    encode_envelope = _encode_envelope
except Exception as exc:  # pragma: no cover - exercised as BLOCKED in deployment
    _PERSISTENCE_IMPORT_ERROR = exc
else:
    _PERSISTENCE_IMPORT_ERROR = None

_EXPECTED_CANDIDATES_CACHE: tuple[list[dict[str, Any]], dict[str, dict[str, str]]] | None = None


class CCheckError(Exception):
    def __init__(self, status: str, code: str, message: str) -> None:
        super().__init__(f"{status} {code}: {message}")
        self.status = status
        self.code = code
        self.message = message


def fail(code: str, message: str) -> NoReturn:
    raise CCheckError("FAIL", code, message)


def blocked(code: str, message: str) -> NoReturn:
    raise CCheckError("BLOCKED", code, message)


def require(condition: bool, code: str, message: str) -> None:
    if not condition:
        fail(code, message)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def json_bytes(value: object) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode("utf-8")


def reject_duplicate_keys(pairs: list[tuple[str, object]]) -> dict[str, object]:
    value: dict[str, object] = {}
    for key, item in pairs:
        if key in value:
            raise ValueError(f"duplicate JSON key {key!r}")
        value[key] = item
    return value


def parse_json(raw: bytes, label: str) -> object:
    try:
        return json.loads(raw.decode("utf-8"), object_pairs_hook=reject_duplicate_keys)
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
        blocked("ARTIFACT_UNREADABLE", f"{label}: {exc}")


def mapping(value: object, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail("SCHEMA_MISMATCH", f"{label} must be an object")
    return value


def string(value: object, label: str) -> str:
    if not isinstance(value, str):
        fail("SCHEMA_MISMATCH", f"{label} must be text")
    return value


def nonempty_string(value: object, label: str) -> str:
    text = string(value, label)
    if not text:
        fail("SCHEMA_MISMATCH", f"{label} must be nonempty")
    return text


def list_value(value: object, label: str) -> list[Any]:
    if not isinstance(value, list):
        fail("SCHEMA_MISMATCH", f"{label} must be an array")
    return value


def exact_keys(value: dict[str, Any], expected: set[str], label: str) -> None:
    if set(value) != expected:
        fail(
            "SCHEMA_MISMATCH",
            f"{label} keys differ: expected {sorted(expected)}, found {sorted(value)}",
        )


def hex64(value: object, label: str) -> str:
    text = string(value, label)
    if HEX64_RE.fullmatch(text) is None:
        fail("SHA256_SCALAR_ENCODING_INVALID", f"{label} is not lowercase 64-hex")
    return text


def uuid(value: object, label: str) -> str:
    text = string(value, label)
    if UUID_RE.fullmatch(text) is None:
        fail("OSI_UNKNOWN", f"{label} is not a lowercase UUID")
    return text


def canonical(value: object) -> bytes:
    if encode_canonical is None:
        blocked("CANONICAL_CODEC_UNAVAILABLE", str(_PERSISTENCE_IMPORT_ERROR))
    return cast(bytes, encode_canonical(value))


def enum(value: object, vocabulary: tuple[str, ...], label: str) -> list[Any]:
    text = string(value, label)
    if text not in vocabulary:
        if text.lower() in vocabulary:
            fail("NONCANONICAL_ENUM_VARIANT", f"{label} uses noncanonical {text!r}")
        fail("VOCABULARY_VARIANT_UNKNOWN", f"{label} has unknown value {text!r}")
    return [text, None]


def enum_value(value: object, vocabulary: tuple[str, ...], label: str) -> str:
    enum(value, vocabulary, label)
    return string(value, label)


def digest_ref(semantic_domain: str, input_schema: str, digest: bytes) -> dict[str, str]:
    if len(digest) != 32:
        fail("CANDIDATE_IDENTITY_MISMATCH", "digest must be 32 bytes")
    return {
        "envelope_id": "mtgml.digest-envelope.v1",
        "algorithm_id": "sha-256",
        "semantic_domain": semantic_domain,
        "payload_codec_id": "mtgml.canonical-cbor.v1",
        "input_schema_id": input_schema,
        "digest_hex": digest.hex(),
    }


def validate_digest_ref(value: object, label: str) -> list[Any]:
    ref = mapping(value, label)
    exact_keys(
        ref,
        {
            "envelope_id",
            "algorithm_id",
            "semantic_domain",
            "payload_codec_id",
            "input_schema_id",
            "digest_hex",
        },
        label,
    )
    require(
        ref["envelope_id"] == "mtgml.digest-envelope.v1",
        "CANDIDATE_IDENTITY_MISMATCH",
        f"{label}.envelope_id",
    )
    require(
        ref["algorithm_id"] == "sha-256", "CANDIDATE_IDENTITY_MISMATCH", f"{label}.algorithm_id"
    )
    require(
        ref["payload_codec_id"] == "mtgml.canonical-cbor.v1",
        "CANDIDATE_IDENTITY_MISMATCH",
        f"{label}.payload_codec_id",
    )
    domain = nonempty_string(ref["semantic_domain"], f"{label}.semantic_domain")
    schema = nonempty_string(ref["input_schema_id"], f"{label}.input_schema_id")
    return [
        ref["envelope_id"],
        ref["algorithm_id"],
        domain,
        ref["payload_codec_id"],
        schema,
        bytes.fromhex(hex64(ref["digest_hex"], f"{label}.digest_hex")),
    ]


def identity_digest(domain: str, schema: str, payload: list[Any]) -> dict[str, str]:
    if encode_envelope is None:
        blocked("CANONICAL_CODEC_UNAVAILABLE", str(_PERSISTENCE_IMPORT_ERROR))
    payload_bytes = canonical(payload)
    envelope_bytes = encode_envelope(domain, schema, payload_bytes)
    return digest_ref(domain, schema, hashlib.sha256(envelope_bytes).digest())


def sha_bytes(value: object, label: str) -> bytes:
    return bytes.fromhex(hex64(value, label))


def canonical_sorted(values: list[Any], label: str, duplicate_code: str) -> list[Any]:
    keys = [canonical(item) for item in values]
    if len(set(keys)) != len(keys):
        fail(duplicate_code, f"{label} contains duplicate canonical values")
    if keys != sorted(keys):
        fail("NONCANONICAL_ORDER", f"{label} is not in canonical order")
    return values


class ArchiveReader:
    def __init__(self, path: Path) -> None:
        if not path.is_file():
            blocked("ARCHIVE_SOURCE_UNAVAILABLE", f"archive not found: {path}")
        raw = path.read_bytes()
        actual = sha256_bytes(raw)
        if actual != EXPECTED_ARCHIVE_SHA256:
            fail(
                "REV3_ARCHIVE_DIGEST_MISMATCH",
                f"archive has {actual}, expected {EXPECTED_ARCHIVE_SHA256}",
            )
        self.path = path
        self.raw = raw
        try:
            self.zip = zipfile.ZipFile(io.BytesIO(raw))
        except zipfile.BadZipFile as exc:
            blocked("ARCHIVE_UNREADABLE", str(exc))
        self.names = set(self.zip.namelist())
        manifest_raw = self.read("Manafold_M2_5_Package_Manifest_REV3.json")
        self.manifest = mapping(
            parse_json(manifest_raw, "REV3 package manifest"), "REV3 package manifest"
        )
        entries = list_value(self.manifest.get("entries"), "manifest.entries")
        self.entry_sha: dict[str, str] = {}
        for item in entries:
            record = mapping(item, "manifest entry")
            path_text = nonempty_string(record.get("path"), "manifest entry.path")
            self.entry_sha[path_text] = hex64(
                record.get("sha256"), f"manifest entry {path_text}.sha256"
            )

    def read(self, name: str) -> bytes:
        if name not in self.names:
            blocked("ARCHIVE_MEMBER_MISSING", f"missing archive member {name}")
        return self.zip.read(name)

    def read_verified(self, name: str) -> bytes:
        raw = self.read(name)
        expected = self.entry_sha.get(name)
        if expected is None:
            fail("ARCHIVE_MEMBER_IDENTITY_MISSING", f"member {name} is absent from the manifest")
        actual = sha256_bytes(raw)
        if actual != expected:
            fail(
                "ARCHIVE_MEMBER_DIGEST_MISMATCH", f"member {name} has {actual}, expected {expected}"
            )
        return raw


def archive_path() -> Path:
    base = os.environ.get(ARCHIVE_ENV_VAR)
    if not base:
        blocked("ARCHIVE_SOURCE_UNAVAILABLE", f"{ARCHIVE_ENV_VAR} is unset")
    root = Path(base).resolve()
    candidate = (root / ARCHIVE_RELATIVE_PATH).resolve()
    try:
        candidate.relative_to(root)
    except ValueError:
        blocked("ARCHIVE_LOCATOR_INVALID", "archive locator escapes the configured archive root")
    return candidate


def load_archive() -> ArchiveReader:
    return ArchiveReader(archive_path())


def load_csv(raw: bytes, label: str) -> list[dict[str, str]]:
    try:
        return list(csv.DictReader(io.StringIO(raw.decode("utf-8"), newline="")))
    except (UnicodeDecodeError, csv.Error) as exc:
        blocked("ARCHIVE_MEMBER_UNREADABLE", f"{label}: {exc}")


def local_raw(name: str) -> bytes:
    path = C_DIR / name
    if not path.is_file():
        blocked("C_ARTIFACT_MISSING", f"missing C artifact {path}")
    return path.read_bytes()


def validate_c_inventory() -> None:
    if not C_DIR.is_dir():
        blocked("C_ARTIFACT_MISSING", f"missing C directory {C_DIR}")
    actual = {path.relative_to(C_DIR).as_posix() for path in C_DIR.rglob("*") if path.is_file()}
    validate_c_file_inventory(actual)
    spec_raw = local_raw("C_DESIGN_SPEC.md")
    require(
        sha256_bytes(spec_raw) == EXPECTED_SPEC_SHA256,
        "C_SPEC_IDENTITY_MISMATCH",
        "approved C_DESIGN_SPEC.md digest",
    )


def validate_c_file_inventory(actual: set[str]) -> None:
    if actual != EXPECTED_C_DIRECTORY_FILES:
        fail(
            "UNLISTED_C_ARTIFACT",
            f"C inventory differs: expected {sorted(EXPECTED_C_DIRECTORY_FILES)}; "
            f"found {sorted(actual)}",
        )


class Snapshot:
    def __init__(self, values: dict[str, object], raw: dict[str, bytes]) -> None:
        self.values = values
        self.raw = raw

    def clone(self) -> Snapshot:
        return Snapshot(copy.deepcopy(self.values), dict(self.raw))


def load_snapshot() -> Snapshot:
    validate_c_inventory()
    values: dict[str, object] = {}
    raw: dict[str, bytes] = {}
    for name in (
        MODEL_NAME,
        REVIEW_NAME,
        UNIVERSE_NAME,
        CLASSES_NAME,
        CLASSIFICATIONS_NAME,
        *SHARD_NAMES,
        CLOSURE_NAME,
        MATRIX_NAME,
        SUMMARY_NAME,
    ):
        data = local_raw(
            name if name != MATRIX_NAME and name != SUMMARY_NAME else f"verification/{name}"
        )
        key = name
        values[key] = parse_json(data, key)
        raw[key] = data
    report = C_DIR / REPORT_NAME
    if not report.is_file():
        blocked("C_ARTIFACT_MISSING", f"missing C report {report}")
    raw[REPORT_NAME] = report.read_bytes()
    values[REPORT_NAME] = raw[REPORT_NAME].decode("utf-8")
    return Snapshot(values, raw)


def git_bytes_at(repo: Path, args: list[str]) -> bytes:
    try:
        return subprocess.run(
            ["git", "-C", str(repo), *args],
            check=True,
            capture_output=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError) as exc:
        blocked("GIT_HISTORY_UNAVAILABLE", f"git -C {repo} {' '.join(args)}: {exc}")


def git_bytes(args: list[str]) -> bytes:
    return git_bytes_at(ROOT, args)


def git_text_at(repo: Path, args: list[str]) -> str:
    return git_bytes_at(repo, args).decode("utf-8")


def git_text(args: list[str]) -> str:
    return git_text_at(ROOT, args)


def tracked_tree_fingerprint(commit: str) -> str:
    paths_raw = git_bytes(["ls-tree", "-r", "-z", "--name-only", commit])
    fingerprint_input = bytearray()
    for path_bytes in paths_raw.split(b"\0"):
        if not path_bytes:
            continue
        try:
            path = path_bytes.decode("utf-8")
        except UnicodeDecodeError as exc:
            blocked("GIT_HISTORY_UNAVAILABLE", f"non-UTF-8 tracked path in {commit}: {exc}")
        payload = git_bytes(["show", f"{commit}:{path}"])
        fingerprint_input.extend(struct.pack(">Q", len(path_bytes)))
        fingerprint_input.extend(path_bytes)
        fingerprint_input.extend(struct.pack(">Q", len(payload)))
        fingerprint_input.extend(payload)
    return sha256_bytes(bytes(fingerprint_input))


def git_commit_parents(commit: str) -> list[str]:
    return git_text(["rev-list", "--parents", "-n", "1", commit]).strip().split()[1:]


def git_diff_name_status(parent: str, commit: str) -> list[str]:
    return git_text(["diff", "--name-status", "--no-renames", parent, commit]).splitlines()


def publishability_preflight(
    checked_commit: str,
    verified_base_commit: str,
    *,
    require_origin_master_identity: bool = False,
    repo: Path = ROOT,
) -> dict[str, Any]:
    """Scan only objects introduced beyond the verified publishing base."""
    repo = repo.resolve()
    if not re.fullmatch(r"[0-9a-f]{40}", checked_commit):
        blocked("PUBLISHABILITY_PRECHECK_INVALID", f"invalid checked commit {checked_commit!r}")
    if not re.fullmatch(r"[0-9a-f]{40}", verified_base_commit):
        blocked(
            "PUBLISHABILITY_PRECHECK_INVALID",
            f"invalid verified base commit {verified_base_commit!r}",
        )
    base_argument = verified_base_commit
    if require_origin_master_identity:
        actual_base = git_text_at(repo, ["rev-parse", "origin/master"]).strip()
        if actual_base != verified_base_commit:
            blocked(
                "PUBLISHABILITY_BASE_MISMATCH",
                f"origin/master is {actual_base}, expected {verified_base_commit}",
            )
        base_argument = "origin/master"
    try:
        subprocess.run(
            [
                "git",
                "-C",
                str(repo),
                "merge-base",
                "--is-ancestor",
                verified_base_commit,
                checked_commit,
            ],
            check=True,
            capture_output=True,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        blocked("PUBLISHABILITY_BASE_NOT_ANCESTOR", f"verified base is not an ancestor: {exc}")
    lines = git_text_at(
        repo, ["rev-list", "--objects", checked_commit, "--not", base_argument]
    ).splitlines()
    object_paths: dict[str, str] = {}
    for line in lines:
        parts = line.split(" ", 1)
        object_id = parts[0]
        if not re.fullmatch(r"[0-9a-f]{40}", object_id):
            blocked("PUBLISHABILITY_OBJECT_UNRESOLVED", f"invalid object id {object_id!r}")
        object_paths.setdefault(object_id, parts[1] if len(parts) == 2 else "")
    object_ids = sorted(object_paths)
    try:
        process = subprocess.run(
            [
                "git",
                "-C",
                str(repo),
                "cat-file",
                "--batch-check=%(objectname) %(objecttype) %(objectsize)",
            ],
            input=("\n".join(object_ids) + "\n" if object_ids else "").encode("ascii"),
            check=True,
            capture_output=True,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        blocked("PUBLISHABILITY_OBJECT_UNRESOLVED", f"git cat-file failed: {exc}")
    resolved: dict[str, tuple[str, int]] = {}
    for raw_line in process.stdout.decode("ascii").splitlines():
        parts = raw_line.split()
        if len(parts) != 3 or parts[0] not in object_paths:
            blocked(
                "PUBLISHABILITY_OBJECT_UNRESOLVED",
                f"unresolvable object response {raw_line!r}",
            )
        try:
            resolved[parts[0]] = (parts[1], int(parts[2]))
        except ValueError as exc:
            blocked(
                "PUBLISHABILITY_OBJECT_UNRESOLVED",
                f"invalid object response {raw_line!r}: {exc}",
            )
    if set(resolved) != set(object_ids):
        blocked("PUBLISHABILITY_OBJECT_UNRESOLVED", "git cat-file did not resolve every object")
    oversized: list[dict[str, Any]] = []
    blob_count = 0
    for object_id in object_ids:
        object_type, size = resolved[object_id]
        if object_type != "blob":
            continue
        blob_count += 1
        if size > HOSTING_HARD_LIMIT_BYTES:
            oversized.append(
                {"object_id": object_id, "path": object_paths[object_id], "size": size}
            )
    legacy_commits = git_text_at(
        repo,
        [
            "rev-list",
            "--full-history",
            checked_commit,
            "--",
            MIGRATION_SOURCE_CLASSIFICATIONS_PATH,
        ]
    ).splitlines()
    forbidden = [MIGRATION_SOURCE_CLASSIFICATIONS_PATH] if legacy_commits else []
    result = "PASS" if not oversized and not forbidden else "FAIL"
    if oversized:
        result = "FAIL"
    return {
        "base_ref": "origin/master",
        "verified_base_commit": verified_base_commit,
        "checked_commit": checked_commit,
        "introduced_object_count": len(object_ids),
        "introduced_blob_count": blob_count,
        "hosting_hard_limit_bytes": HOSTING_HARD_LIMIT_BYTES,
        "oversized_blobs": oversized,
        "forbidden_legacy_paths": forbidden,
        "result": result,
    }


def validate_historical_evidence_chain(summary: dict[str, Any], current_summary_raw: bytes) -> None:
    execution_commit = string(summary["execution_commit"], "verification summary.execution_commit")
    protocol = mapping(summary["evidence_protocol"], "verification summary.evidence_protocol")
    require(
        protocol["H_exec"] == execution_commit,
        "SOURCE_CHANGED_AFTER_H_EXEC",
        "summary H_exec differs from execution_commit",
    )
    head = git_text(["rev-parse", "HEAD"]).strip()
    if head == execution_commit:
        fail(
            "SOURCE_CHANGED_AFTER_H_EXEC", "a final evidence summary cannot be committed as H_exec"
        )
    try:
        subprocess.run(
            ["git", "-C", str(ROOT), "merge-base", "--is-ancestor", execution_commit, head],
            check=True,
            capture_output=True,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        fail("SOURCE_CHANGED_AFTER_H_EXEC", f"H_exec is not an ancestor of current HEAD: {exc}")

    summary_path = "sources/m2_5/closures/C/verification/c_verification_summary.v3.json"
    candidates: list[str] = []
    for commit in git_text(["rev-list", head]).splitlines():
        parents = git_commit_parents(commit)
        if parents != [execution_commit]:
            continue
        try:
            candidate_raw = git_bytes(["show", f"{commit}:{summary_path}"])
        except CCheckError:
            continue
        try:
            candidate = mapping(
                parse_json(candidate_raw, f"historical summary at {commit}"), "historical summary"
            )
        except CCheckError:
            continue
        if candidate.get("execution_commit") != execution_commit:
            continue
        candidate_protocol = candidate.get("evidence_protocol")
        if (
            not isinstance(candidate_protocol, dict)
            or candidate_protocol.get("H_exec") != execution_commit
            or candidate_protocol.get("H_evidence_relation") != "direct_child_summary_only"
        ):
            continue
        if git_diff_name_status(execution_commit, commit) != [f"M\t{summary_path}"]:
            continue
        if candidate_raw != current_summary_raw:
            continue
        candidates.append(commit)
    if len(candidates) != 1:
        fail(
            "SOURCE_CHANGED_AFTER_H_EXEC",
            f"expected one reachable summary-only H_evidence commit, found {candidates}",
        )
    evidence_commit = candidates[0]
    try:
        subprocess.run(
            ["git", "-C", str(ROOT), "merge-base", "--is-ancestor", evidence_commit, head],
            check=True,
            capture_output=True,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        fail("SOURCE_CHANGED_AFTER_H_EXEC", f"H_evidence is not an ancestor of current HEAD: {exc}")
    preflight = validate_publishability_preflight_for_history(
        evidence_commit, summary["publishability_preflight"]
    )
    if preflight["result"] != "PASS":
        fail(
            "PUBLISHABILITY_PRECHECK_INVALID",
            f"post-commit H_evidence publishability failed: {preflight}",
        )


def validate_publishability_preflight_for_history(
    checked_commit: str, summary_preflight: object
) -> dict[str, Any]:
    value = mapping(summary_preflight, "publishability preflight")
    base = nonempty_string(value.get("verified_base_commit"), "publishability verified base")
    return publishability_preflight(checked_commit, base)


def set_artifact(snapshot: Snapshot, name: str, value: object) -> None:
    snapshot.values[name] = value
    snapshot.raw[name] = json_bytes(value)


def get_artifact(snapshot: Snapshot, name: str) -> dict[str, Any]:
    return mapping(snapshot.values[name], name)


def source_columns() -> list[str]:
    return [
        "candidate_id",
        "model_id",
        "scope",
        "pair_id",
        "left_family_id",
        "right_family_id",
        "relation",
        "disposition",
        "disposition_reason",
        "supporting_requirement_ids",
    ]


def source_origin_cbor(value: str) -> list[Any]:
    return enum(value, EXTRA_VOCABULARY["source_origin"], "source_origin")


def participant_ref_cbor(value: object, label: str) -> list[Any]:
    item = mapping(value, label)
    exact_keys(item, {"participant_kind", "semantic_ref"}, label)
    kind = enum_value(item["participant_kind"], PARTICIPANT_KINDS, f"{label}.participant_kind")
    reference = nonempty_string(item["semantic_ref"], f"{label}.semantic_ref")
    return [enum(kind, PARTICIPANT_KINDS, f"{label}.participant_kind"), reference]


def participant_binding_cbor(value: object, label: str) -> list[Any]:
    item = mapping(value, label)
    exact_keys(item, {"role", "participant_ref"}, label)
    role = enum_value(item["role"], PARTICIPANT_ROLES, f"{label}.role")
    return [
        enum(role, PARTICIPANT_ROLES, f"{label}.role"),
        participant_ref_cbor(item["participant_ref"], f"{label}.participant_ref"),
    ]


def context_cbor(value: object, label: str) -> list[Any]:
    item = mapping(value, label)
    if set(item) != set(CONTEXT_KEYS):
        if set(item) < set(CONTEXT_KEYS):
            fail("CONTEXT_DIMENSION_MISSING", f"{label} is missing a declared dimension")
        fail("VOCABULARY_VARIANT_UNKNOWN", f"{label} contains an undeclared dimension")
    result: list[Any] = []
    for key in CONTEXT_KEYS:
        result.append(enum(item[key], CONTEXT_VOCABULARY[key], f"{label}.{key}"))
    return result


def temporal_cbor(value: object, label: str) -> list[Any]:
    item = mapping(value, label)
    exact_keys(item, set(TEMPORAL_KEYS), label)
    return [enum(item[key], TEMPORAL_VOCABULARY[key], f"{label}.{key}") for key in TEMPORAL_KEYS]


def evidence_ref_cbor(value: object, label: str) -> list[Any]:
    item = mapping(value, label)
    exact_keys(item, {"authority_kind", "path", "locator", "raw_sha256"}, label)
    authority = enum_value(
        item["authority_kind"], EXTRA_VOCABULARY["authority_kind"], f"{label}.authority_kind"
    )
    path = nonempty_string(item["path"], f"{label}.path")
    locator = item["locator"]
    try:
        canonical(locator)
    except (TypeError, ValueError) as exc:
        fail("SCHEMA_MISMATCH", f"{label}.locator is not an allowed canonical-CBOR value: {exc}")
    sha = sha_bytes(item["raw_sha256"], f"{label}.raw_sha256")
    return [
        enum(authority, EXTRA_VOCABULARY["authority_kind"], f"{label}.authority_kind"),
        path,
        locator,
        sha,
    ]


def evidence_sort(values: list[Any], label: str) -> list[Any]:
    keys = [canonical(evidence_ref_cbor(value, f"{label}[{i}]")) for i, value in enumerate(values)]
    if len(set(keys)) != len(keys):
        fail("DUPLICATE_EVIDENCE_REF", f"{label} contains duplicate refs")
    if keys != sorted(keys):
        fail("NONCANONICAL_ORDER", f"{label} is not canonically ordered")
    return values


def b1_citation_cbor(value: object, label: str) -> list[Any]:
    item = mapping(value, label)
    exact_keys(item, {"authority_id", "citation_id"}, label)
    return [
        nonempty_string(item["authority_id"], f"{label}.authority_id"),
        nonempty_string(item["citation_id"], f"{label}.citation_id"),
    ]


def b1_citation_sort(values: list[Any], label: str) -> list[Any]:
    keys = [canonical(b1_citation_cbor(value, f"{label}[{i}]")) for i, value in enumerate(values)]
    if len(set(keys)) != len(keys):
        fail("B1_CITATION_UNRESOLVED", f"{label} contains duplicate citations")
    if keys != sorted(keys):
        fail("NONCANONICAL_ORDER", f"{label} is not canonically ordered")
    return values


def source_binding_cbor(value: object, label: str) -> list[Any]:
    item = mapping(value, label)
    kind = enum_value(item.get("kind"), ("rev3", "targeted_higher_order_review"), f"{label}.kind")
    if kind == "rev3":
        exact_keys(
            item,
            {
                "kind",
                "archive_member",
                "archive_member_sha256",
                "row_ordinal",
                "source_columns",
                "source_values",
            },
            label,
        )
        columns = list_value(item["source_columns"], f"{label}.source_columns")
        values = list_value(item["source_values"], f"{label}.source_values")
        require(
            columns == source_columns(),
            "SOURCE_BINDING_INVALID",
            f"{label}.source_columns mismatch",
        )
        require(
            len(values) == len(columns) and all(isinstance(x, str) for x in values),
            "SOURCE_BINDING_INVALID",
            f"{label}.source_values invalid",
        )
        row_ordinal = item["row_ordinal"]
        if not isinstance(row_ordinal, int) or isinstance(row_ordinal, bool) or row_ordinal < 0:
            fail("SOURCE_BINDING_INVALID", f"{label}.row_ordinal is not a nonnegative integer")
        return [
            enum(kind, ("rev3", "targeted_higher_order_review"), f"{label}.kind"),
            [
                nonempty_string(item["archive_member"], f"{label}.archive_member"),
                sha_bytes(item["archive_member_sha256"], f"{label}.archive_member_sha256"),
                row_ordinal,
                values_for_cbor(columns),
                values_for_cbor(values),
            ],
        ]
    exact_keys(
        item,
        {
            "kind",
            "additions_path",
            "additions_raw_sha256",
            "review_record_id",
            "review_kind",
            "participant_source_refs",
            "review_evidence_refs",
        },
        label,
    )
    refs = list_value(item["participant_source_refs"], f"{label}.participant_source_refs")
    evidence = evidence_sort(
        list_value(item["review_evidence_refs"], f"{label}.review_evidence_refs"),
        f"{label}.review_evidence_refs",
    )
    return [
        enum(kind, ("rev3", "targeted_higher_order_review"), f"{label}.kind"),
        [
            nonempty_string(item["additions_path"], f"{label}.additions_path"),
            sha_bytes(item["additions_raw_sha256"], f"{label}.additions_raw_sha256"),
            nonempty_string(item["review_record_id"], f"{label}.review_record_id"),
            enum(item["review_kind"], EXTRA_VOCABULARY["review_kind"], f"{label}.review_kind"),
            [
                participant_source_ref_cbor(x, f"{label}.participant_source_refs[{i}]")
                for i, x in enumerate(refs)
            ],
            [
                evidence_ref_cbor(x, f"{label}.review_evidence_refs[{i}]")
                for i, x in enumerate(evidence)
            ],
        ],
    ]


def values_for_cbor(values: list[Any]) -> list[Any]:
    result: list[Any] = []
    for value in values:
        if isinstance(value, str):
            result.append(value)
        else:
            result.append(value)
    return result


def participant_source_ref_cbor(value: object, label: str) -> list[Any]:
    item = mapping(value, label)
    exact_keys(item, {"source_kind", "source_locator"}, label)
    kind = enum_value(
        item["source_kind"],
        ("b2_assignment", "b2_classification", "rev3_row"),
        f"{label}.source_kind",
    )
    return [
        enum(kind, ("b2_assignment", "b2_classification", "rev3_row"), f"{label}.source_kind"),
        item["source_locator"],
    ]


def candidate_identity_payload(candidate: dict[str, Any]) -> list[Any]:
    return [
        enum(
            candidate["source_origin"], EXTRA_VOCABULARY["source_origin"], "candidate.source_origin"
        ),
        enum(candidate["scope"], EXTRA_VOCABULARY["scope"], "candidate.scope"),
        enum(candidate["relation"], EXTRA_VOCABULARY["relation"], "candidate.relation"),
        [
            participant_ref_cbor(x, "candidate.participant_refs")
            for x in candidate["participant_refs"]
        ],
        [
            x
            for x in canonical_sorted(
                list(candidate["supporting_requirement_ids"]),
                "candidate.supporting_requirement_ids",
                "DUPLICATE_REQUIREMENT_ID",
            )
        ],
        source_binding_cbor(candidate["source_binding"], "candidate.source_binding"),
    ]


def class_identity_payload(record: dict[str, Any]) -> list[Any]:
    roles = list_value(record["participant_roles"], "class.participant_roles")
    role_payloads: list[Any] = []
    for i, role in enumerate(roles):
        item = mapping(role, f"class.participant_roles[{i}]")
        exact_keys(
            item,
            {"position", "role", "participant_kind", "semantic_ref"},
            f"class.participant_roles[{i}]",
        )
        role_payloads.append(
            [
                item["position"],
                enum(item["role"], PARTICIPANT_ROLES, f"class.participant_roles[{i}].role"),
                enum(
                    item["participant_kind"],
                    PARTICIPANT_KINDS,
                    f"class.participant_roles[{i}].participant_kind",
                ),
                nonempty_string(item["semantic_ref"], f"class.participant_roles[{i}].semantic_ref"),
            ]
        )
    b2_families: list[Any] = []
    for i, ref in enumerate(record["b2_family_refs"]):
        item = mapping(ref, f"class.b2_family_refs[{i}]")
        exact_keys(
            item, {"family_id", "lifecycle", "assignment_role"}, f"class.b2_family_refs[{i}]"
        )
        b2_families.append(
            [
                nonempty_string(item["family_id"], f"class.b2_family_refs[{i}].family_id"),
                enum(
                    item["lifecycle"],
                    EXTRA_VOCABULARY["lifecycle"],
                    f"class.b2_family_refs[{i}].lifecycle",
                ),
                enum(
                    item["assignment_role"],
                    EXTRA_VOCABULARY["assignment_role"],
                    f"class.b2_family_refs[{i}].assignment_role",
                ),
            ]
        )
    b2_boundaries: list[Any] = []
    for i, ref in enumerate(record["b2_boundary_refs"]):
        item = mapping(ref, f"class.b2_boundary_refs[{i}]")
        exact_keys(
            item, {"family_id", "precise_semantic_definition"}, f"class.b2_boundary_refs[{i}]"
        )
        b2_boundaries.append(
            [
                nonempty_string(item["family_id"], f"class.b2_boundary_refs[{i}].family_id"),
                nonempty_string(
                    item["precise_semantic_definition"],
                    f"class.b2_boundary_refs[{i}].precise_semantic_definition",
                ),
            ]
        )
    citations = b1_citation_sort(
        list_value(record["b1_final_citation_refs"], "class.b1_final_citation_refs"),
        "class.b1_final_citation_refs",
    )
    return [
        enum(record["arity"], EXTRA_VOCABULARY["arity"], "class.arity"),
        enum(record["directionality"], EXTRA_VOCABULARY["directionality"], "class.directionality"),
        role_payloads,
        enum(
            record["host_relationship"],
            EXTRA_VOCABULARY["host_relationship"],
            "class.host_relationship",
        ),
        context_cbor(record["context_dimensions"], "class.context_dimensions"),
        temporal_cbor(record["temporal_semantics"], "class.temporal_semantics"),
        canonical_sorted(b2_families, "class.b2_family_refs", "DUPLICATE_B2_REFERENCE"),
        canonical_sorted(b2_boundaries, "class.b2_boundary_refs", "DUPLICATE_B2_REFERENCE"),
        [b1_citation_cbor(x, "class.b1_final_citation_refs") for x in citations],
    ]


def make_candidate_identity(candidate: dict[str, Any]) -> dict[str, str]:
    return identity_digest(
        "manafold.m2.5.c.candidate-identity.v1",
        "manafold.m2.5.c.candidate-identity-input.v1",
        candidate_identity_payload(candidate),
    )


def make_class_identity(record: dict[str, Any]) -> dict[str, str]:
    return identity_digest(
        "manafold.m2.5.c.interaction-class-identity.v1",
        "manafold.m2.5.c.interaction-class-identity-input.v1",
        class_identity_payload(record),
    )


def raw_binding(path: str, raw: bytes) -> dict[str, str]:
    return {"path": path, "raw_sha256": sha256_bytes(raw)}


def b64_candidate_id(candidate_id: str) -> str:
    return base64.urlsafe_b64encode(candidate_id.encode("utf-8")).decode("ascii").rstrip("=")


def parse_supporting_ids(
    row: dict[str, str], trigger_osi: str | None, family_ids: set[str]
) -> list[str]:
    cell = row["supporting_requirement_ids"]
    if not cell or cell.strip() != cell:
        fail("SOURCE_CELL_PARSE_INVALID", f"supporting_requirement_ids is not exact JSON: {cell!r}")
    try:
        parsed = json.loads(cell, object_pairs_hook=reject_duplicate_keys)
    except (json.JSONDecodeError, ValueError) as exc:
        fail("SOURCE_CELL_PARSE_INVALID", f"supporting_requirement_ids: {exc}")
    if not isinstance(parsed, list) or any(
        not isinstance(item, str) or not item for item in parsed
    ):
        fail(
            "SOURCE_CELL_PARSE_INVALID",
            "supporting_requirement_ids must be a string array",
        )
    if len(set(parsed)) != len(parsed):
        fail("DUPLICATE_REQUIREMENT_ID", "supporting_requirement_ids contains duplicates")
    if trigger_osi is None:
        if any(item not in family_ids for item in parsed):
            fail("FAMILY_UNKNOWN", "supporting_requirement_ids contains an unknown B2 family")
    elif parsed != [trigger_osi]:
        fail(
            "SOURCE_CELL_PARSE_INVALID",
            "card trigger supporting requirement does not equal joined OSI",
        )
    return sorted(parsed, key=lambda item: canonical(item))


def normalized_participants(
    row: dict[str, str], family_ids: set[str], trigger_osi: str | None
) -> list[dict[str, str]]:
    if trigger_osi is not None:
        return [{"participant_kind": "card", "semantic_ref": trigger_osi}]
    left = row["left_family_id"]
    right = row["right_family_id"]
    if left not in family_ids or right not in family_ids:
        fail("FAMILY_UNKNOWN", f"candidate references unknown family {left!r}/{right!r}")
    refs = [
        {"participant_kind": "requirement_family", "semantic_ref": left},
        {"participant_kind": "requirement_family", "semantic_ref": right},
    ]
    if row["relation"] == "UNORDERED_BINARY":
        require(
            left.encode("utf-8") <= right.encode("utf-8"),
            "SOURCE_ROW_ORDER_INVALID",
            "unordered source family order is invalid",
        )
        refs.sort(key=lambda item: canonical(participant_ref_cbor(item, "participant")))
    return refs


def source_binding_from_row(row: dict[str, str], ordinal: int, archive_sha: str) -> dict[str, Any]:
    return {
        "kind": "rev3",
        "archive_member": EXPECTED_REV3_CENSUS_MEMBER,
        "archive_member_sha256": archive_sha,
        "row_ordinal": ordinal,
        "source_columns": source_columns(),
        "source_values": [row[column] for column in source_columns()],
    }


def context_not_applicable() -> dict[str, str]:
    return {key: "not_applicable" for key in CONTEXT_KEYS}


def candidate_instance(candidate: dict[str, Any], instance_id: str) -> dict[str, Any]:
    participant_roles = (
        "trigger_source"
        if candidate["relation"] == "declared_card_trigger"
        else "ordered_participant"
    )
    return {
        "source_instance_id": instance_id,
        "candidate_id": candidate["candidate_id"],
        "source_binding": copy.deepcopy(candidate["source_binding"]),
        "participant_bindings": [
            {"role": participant_roles, "participant_ref": copy.deepcopy(ref)}
            for ref in candidate["participant_refs"]
        ],
        "source_context": context_not_applicable(),
    }


def rev3_rows(reader: ArchiveReader) -> list[dict[str, str]]:
    raw = reader.read_verified(EXPECTED_REV3_CENSUS_MEMBER)
    if sha256_bytes(raw) != EXPECTED_REV3_CENSUS_SHA256:
        fail(
            "REV3_CANDIDATE_SOURCE_DIGEST_MISMATCH", "Pair Interaction Census digest is not pinned"
        )
    rows = load_csv(raw, EXPECTED_REV3_CENSUS_MEMBER)
    require(
        bool(rows) and list(rows[0]) == source_columns(),
        "REV3_SOURCE_SCHEMA_MISMATCH",
        "census columns differ from C V1",
    )
    require(
        len(rows) == EXPECTED_REV3_CANDIDATE_COUNT,
        "REV3_CANDIDATE_COUNT_MISMATCH",
        "pinned REV3 candidate count",
    )
    for ordinal, row in enumerate(rows):
        if set(row) != set(source_columns()) or any(value is None for value in row.values()):
            fail("REV3_SOURCE_SCHEMA_MISMATCH", f"census row {ordinal} has an invalid column set")
    return rows


def resolution_joins(
    reader: ArchiveReader,
) -> tuple[dict[str, list[dict[str, str]]], dict[tuple[str, str], list[dict[str, str]]]]:
    resolutions = load_csv(
        reader.read_verified(EXPECTED_RESOLUTION_MEMBER), EXPECTED_RESOLUTION_MEMBER
    )
    index = load_csv(reader.read_verified(EXPECTED_INDEX_MEMBER), EXPECTED_INDEX_MEMBER)
    by_osi: dict[str, list[dict[str, str]]] = {}
    for row in resolutions:
        by_osi.setdefault(row["oracle_semantic_identity"], []).append(row)
    by_pair: dict[tuple[str, str], list[dict[str, str]]] = {}
    for row in index:
        if row["oracle_semantic_identity"]:
            by_pair.setdefault(
                (row["oracle_semantic_identity"], row["source_record_id"]), []
            ).append(row)
    return by_osi, by_pair


def expected_candidates(
    reader: ArchiveReader, catalog: dict[str, Any]
) -> tuple[list[dict[str, Any]], dict[str, dict[str, str]]]:
    global _EXPECTED_CANDIDATES_CACHE
    if _EXPECTED_CANDIDATES_CACHE is not None:
        return _EXPECTED_CANDIDATES_CACHE
    families = list_value(catalog.get("families"), "B2 families")
    family_ids = {nonempty_string(item.get("family_id"), "B2 family_id") for item in families}
    by_osi, by_pair = resolution_joins(reader)
    result: list[dict[str, Any]] = []
    instance_ids: dict[str, dict[str, str]] = {}
    shape_counts: Counter[tuple[str, str]] = Counter()
    for ordinal, row in enumerate(rev3_rows(reader)):
        require(
            row["model_id"] == EXPECTED_REV3_MODEL_ID,
            "REV3_MODEL_ID_MISMATCH",
            f"row {ordinal} model ID",
        )
        scope_map = {
            "INTRA_DECK": "intra_deck",
            "CROSS_DECK": "cross_deck",
            "UNARY_OR_HIGHER_ORDER": "unary_or_higher_order",
        }
        relation_map = {
            "UNORDERED_BINARY": "unordered_binary",
            "DIRECTIONAL_BINARY": "directional_binary",
            "DECLARED_CARD_TRIGGER": "declared_card_trigger",
        }
        if row["scope"] not in scope_map or row["relation"] not in relation_map:
            fail(
                "NONCANONICAL_ENUM_VARIANT",
                f"row {ordinal} has an unknown historical scope/relation",
            )
        if (row["scope"], row["relation"]) not in {
            ("INTRA_DECK", "UNORDERED_BINARY"),
            ("CROSS_DECK", "DIRECTIONAL_BINARY"),
            ("UNARY_OR_HIGHER_ORDER", "DECLARED_CARD_TRIGGER"),
        }:
            fail("REV3_SHAPE_MISMATCH", f"row {ordinal} has an unsupported relation shape")
        shape_counts[(row["scope"], row["relation"])] += 1
        trigger_osi: str | None = None
        if row["relation"] == "DECLARED_CARD_TRIGGER":
            require(
                row["pair_id"] == row["left_family_id"] == row["right_family_id"],
                "REV3_TRIGGER_JOIN_INVALID",
                f"row {ordinal} trigger IDs differ",
            )
            trigger_osi = row["pair_id"]
            selected = by_osi.get(trigger_osi, [])
            require(
                len(selected) == 1,
                "REV3_TRIGGER_JOIN_INVALID",
                f"OSI {trigger_osi} does not join exactly once",
            )
            resolution = selected[0]
            source_record_id = resolution["oracle_source_record_id"]
            require(
                len(by_pair.get((trigger_osi, source_record_id), [])) == 1,
                "REV3_TRIGGER_JOIN_INVALID",
                f"OSI {trigger_osi} raw source join is not unique",
            )
            uuid(trigger_osi, f"row {ordinal}.oracle_semantic_identity")
        participants = normalized_participants(row, family_ids, trigger_osi)
        supports = parse_supporting_ids(row, trigger_osi, family_ids)
        candidate = {
            "candidate_id": row["candidate_id"],
            "candidate_identity": None,
            "source_origin": "rev3",
            "scope": scope_map[row["scope"]],
            "relation": relation_map[row["relation"]],
            "participant_refs": participants,
            "supporting_requirement_ids": supports,
            "source_binding": source_binding_from_row(
                row, ordinal, reader.entry_sha[EXPECTED_REV3_CENSUS_MEMBER]
            ),
            "reconciliation_status": "unchanged",
            "reconciliation_reason": (
                "Inherited byte-for-byte from the pinned REV3 census; "
                "no C V1 lineage delta applies."
            ),
        }
        candidate["candidate_identity"] = make_candidate_identity(candidate)
        result.append(candidate)
        instance_id = f"si.v1/{b64_candidate_id(row['candidate_id'])}/0"
        instance_ids[row["candidate_id"]] = {"source_instance_id": instance_id}
    require(
        shape_counts
        == Counter(
            {
                ("INTRA_DECK", "UNORDERED_BINARY"): 8131,
                ("CROSS_DECK", "DIRECTIONAL_BINARY"): 7530,
                ("UNARY_OR_HIGHER_ORDER", "DECLARED_CARD_TRIGGER"): 18,
            }
        ),
        "REV3_SHAPE_COUNT_MISMATCH",
        f"REV3 shape counts are {shape_counts}",
    )
    _EXPECTED_CANDIDATES_CACHE = (result, instance_ids)
    return _EXPECTED_CANDIDATES_CACHE


def model_value() -> dict[str, Any]:
    return {
        "schema": C_JSON_SCHEMAS[MODEL_NAME],
        "model_id": MODEL_ID,
        "model_version": MODEL_VERSION,
        "coverage_scope": "pairwise_plus_review_outliers",
        "accepted_rev3_model": EXPECTED_REV3_MODEL_ID,
        "accepted_rev3_candidate_source": EXPECTED_REV3_CENSUS_MEMBER,
        "included_shapes": [
            "unary_card_specific_declared_outliers",
            "binary_family_relations",
            "directional_binary_relations",
            "explicit_reviewed_higher_order_interactions",
        ],
        "excluded_claims": ["arbitrary_unbounded_n_way_magic_interaction_completeness"],
        "terminal_dispositions": list(EXTRA_VOCABULARY["terminal_disposition"]),
        "review_states": list(REVIEW_STATES),
        "unresolved_reasons": list(UNRESOLVED_REASONS),
        "review_domain_vocabulary": list(REVIEW_DOMAINS),
        "review_domain_applicability_vocabulary": list(REVIEW_DOMAIN_APPLICABILITY),
        "context_dimensions": list(CONTEXT_KEYS),
        "authority_policy": (
            "C is additive source-grounded evidence only; B1.Final and B2 "
            "remain immutable authorities."
        ),
        "participant_kind_vocabulary": list(PARTICIPANT_KINDS),
        "participant_role_vocabulary": list(PARTICIPANT_ROLES),
        "context_value_vocabulary": {
            key: list(values) for key, values in CONTEXT_VOCABULARY.items()
        },
        "temporal_value_vocabulary": {
            key: list(values) for key, values in TEMPORAL_VOCABULARY.items()
        },
    }


def empty_review(model_raw: bytes) -> dict[str, Any]:
    return {
        "schema": C_JSON_SCHEMAS[REVIEW_NAME],
        "model_id": MODEL_ID,
        "input_bindings": {
            "declared_model_path": "sources/m2_5/closures/C/declared_interaction_model.v2.json",
            "declared_model_raw_sha256": sha256_bytes(model_raw),
            "source_evidence_refs_sorted_array": [],
        },
        "review_record_count": 0,
        "review_records": [],
    }


def b2_file_records() -> list[dict[str, str]]:
    result: list[dict[str, str]] = []
    for relative in (
        "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
        "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
        "sources/m2_5/closures/B2/classification_closure.v1.json",
    ):
        result.append(raw_binding(relative, (ROOT / relative).read_bytes()))
    return result


def b1_final_file_records() -> list[dict[str, str]]:
    result: list[dict[str, str]] = []
    for relative in (
        "sources/m2_5/closures/B1/official_authority_citations.v3.json",
        "sources/m2_5/closures/B1/official_authority_citation_closure.v2.json",
    ):
        result.append(raw_binding(relative, (ROOT / relative).read_bytes()))
    return result


def make_universe(
    model_raw: bytes,
    review_raw: bytes,
    reader: ArchiveReader,
    candidates: list[dict[str, Any]],
    instances: dict[str, dict[str, str]],
) -> dict[str, Any]:
    source_instances: list[dict[str, Any]] = []
    for candidate in candidates:
        instance_id = instances[candidate["candidate_id"]]["source_instance_id"]
        source_instances.append(candidate_instance(candidate, instance_id))
    return {
        "schema": C_JSON_SCHEMAS[UNIVERSE_NAME],
        "model_id": MODEL_ID,
        "input_bindings": {
            "declared_model": raw_binding(
                "sources/m2_5/closures/C/declared_interaction_model.v2.json", model_raw
            ),
            "review_additions": raw_binding(
                "sources/m2_5/closures/C/interaction_review_additions.v2.json", review_raw
            ),
            "rev3_candidate_source": {
                "archive_member": EXPECTED_REV3_CENSUS_MEMBER,
                "archive_member_sha256": reader.entry_sha[EXPECTED_REV3_CENSUS_MEMBER],
                "source_package_sha256": EXPECTED_ARCHIVE_SHA256,
            },
            "b2_artifacts": b2_file_records(),
            "b1_final_artifacts": b1_final_file_records(),
        },
        "candidate_count": len(candidates),
        "candidate_reconciliation_counts": {
            "unchanged": len(candidates),
            "stale_rev3_candidate": 0,
            "removed_not_interaction": 0,
            "merged_semantic_duplicate": 0,
            "new_targeted_higher_order_candidate": 0,
            "new_b2_derived": 0,
        },
        "source_instance_count": len(source_instances),
        "candidates": candidates,
        "source_instances": source_instances,
    }


def b2_family_maps(catalog: dict[str, Any]) -> tuple[dict[str, dict[str, Any]], set[str], set[str]]:
    families = list_value(catalog.get("families"), "B2 families")
    by_id: dict[str, dict[str, Any]] = {}
    for item in families:
        record = mapping(item, "B2 family")
        family_id = nonempty_string(record.get("family_id"), "B2 family_id")
        if family_id in by_id:
            fail("FAMILY_UNKNOWN", f"duplicate B2 family {family_id}")
        by_id[family_id] = record
    active = {key for key, value in by_id.items() if value.get("status") == "ACTIVE"}
    active_unassigned = {
        key for key, value in by_id.items() if value.get("status") == "ACTIVE_UNASSIGNED"
    }
    return by_id, active, active_unassigned


def b2_classification_map() -> dict[str, dict[str, Any]]:
    raw = (ROOT / "sources/m2_5/closures/B2/card_semantic_classifications.v1.json").read_bytes()
    value = mapping(parse_json(raw, "B2 classifications"), "B2 classifications")
    return {
        nonempty_string(item.get("oracle_semantic_identity"), "B2 OSI"): item
        for item in list_value(value.get("classifications"), "B2 classifications")
    }


def class_evidence_ref(oid: str, row_ordinal: int, b2_raw_sha: str) -> list[dict[str, Any]]:
    values: list[dict[str, Any]] = [
        {
            "authority_kind": "rev3",
            "path": EXPECTED_REV3_CENSUS_MEMBER,
            "locator": ["row_ordinal", row_ordinal],
            "raw_sha256": EXPECTED_REV3_CENSUS_SHA256,
        },
        {
            "authority_kind": "b2",
            "path": "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
            "locator": ["oracle_semantic_identity", oid],
            "raw_sha256": b2_raw_sha,
        },
    ]
    return sorted(values, key=lambda item: canonical(evidence_ref_cbor(item, "class evidence")))


def b2_refs_for_osi(
    oid: str, catalog_by_id: dict[str, dict[str, Any]], b2_by_osi: dict[str, dict[str, Any]]
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    classification = b2_by_osi.get(oid)
    if classification is None:
        fail("OSI_UNKNOWN", f"missing B2 classification for {oid}")
    assignments = list_value(classification.get("requirement_assignments"), f"B2 assignments {oid}")
    family_refs: list[dict[str, Any]] = []
    boundary_refs: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, assignment in enumerate(assignments):
        item = mapping(assignment, f"B2 assignment {oid}[{index}]")
        family_id = nonempty_string(item.get("requirement_family_id"), "B2 assignment family")
        if family_id in seen:
            fail("ASSIGNMENT_BINDING_INVALID", f"duplicate assignment family {family_id} for {oid}")
        seen.add(family_id)
        family = catalog_by_id.get(family_id)
        if family is None:
            fail("FAMILY_UNKNOWN", f"B2 assignment references {family_id}")
        if family.get("status") != "ACTIVE":
            fail("ACTIVE_UNASSIGNED_CARD_DERIVED", f"B2 assignment {family_id} is not ACTIVE")
        family_refs.append(
            {
                "family_id": family_id,
                "lifecycle": "active",
                "assignment_role": "primary" if index == 0 else "supporting",
            }
        )
        boundary_refs.append(
            {
                "family_id": family_id,
                "precise_semantic_definition": nonempty_string(
                    family.get("precise_semantic_definition"), f"B2 boundary {family_id}"
                ),
            }
        )
    family_refs.sort(
        key=lambda item: canonical(
            [
                item["family_id"],
                enum(item["lifecycle"], EXTRA_VOCABULARY["lifecycle"], "lifecycle"),
                enum(
                    item["assignment_role"], EXTRA_VOCABULARY["assignment_role"], "assignment_role"
                ),
            ]
        )
    )
    boundary_refs.sort(
        key=lambda item: canonical([item["family_id"], item["precise_semantic_definition"]])
    )
    return family_refs, boundary_refs


def make_card_class(
    candidate: dict[str, Any],
    row_ordinal: int,
    catalog_by_id: dict[str, dict[str, Any]],
    b2_by_osi: dict[str, dict[str, Any]],
    b2_raw_sha: str,
) -> dict[str, Any]:
    oid = candidate["participant_refs"][0]["semantic_ref"]
    family_refs, boundary_refs = b2_refs_for_osi(oid, catalog_by_id, b2_by_osi)
    record: dict[str, Any] = {
        "interaction_class_id": "",
        "class_identity": None,
        "arity": "unary",
        "directionality": "none",
        "participant_roles": [
            {
                "position": 0,
                "role": "trigger_source",
                "participant_kind": "card",
                "semantic_ref": oid,
            }
        ],
        "host_relationship": "not_applicable",
        "context_dimensions": context_not_applicable(),
        "temporal_semantics": {
            "trigger_order": "immediate",
            "dependency_order": "no_temporal_dependency",
            "duration": "not_applicable",
            "replacement_order": "not_applicable",
        },
        "b2_family_refs": family_refs,
        "b2_boundary_refs": boundary_refs,
        "b1_final_citation_refs": [
            {"authority_id": "comprehensive_rules", "citation_id": "CR-603-triggered-abilities"}
        ],
        "semantic_rationale": (
            "The exact REV3 DECLARED_CARD_TRIGGER row joins one pinned OSI. "
            "Its terminal B2 assignments, exact boundaries, and the accepted "
            "CR-603 citation establish the reviewed trigger source without "
            "deriving meaning from a name, keyword, or candidate text."
        ),
        "source_evidence_refs": class_evidence_ref(oid, row_ordinal, b2_raw_sha),
    }
    identity = make_class_identity(record)
    record["class_identity"] = identity
    record["interaction_class_id"] = f"ic.v1/{identity['digest_hex']}"
    return record


def make_classes(candidates: list[dict[str, Any]], catalog: dict[str, Any]) -> list[dict[str, Any]]:
    catalog_by_id, _, _ = b2_family_maps(catalog)
    b2_by_osi = b2_classification_map()
    b2_raw_sha = sha256_bytes(
        (ROOT / "sources/m2_5/closures/B2/card_semantic_classifications.v1.json").read_bytes()
    )
    return [
        make_card_class(
            candidate,
            candidate["source_binding"]["row_ordinal"],
            catalog_by_id,
            b2_by_osi,
            b2_raw_sha,
        )
        for candidate in candidates
        if candidate["relation"] == "declared_card_trigger"
    ]


def candidate_review_evidence(
    candidate: dict[str, Any], catalog: dict[str, Any]
) -> list[dict[str, Any]]:
    """Return the exact available evidence for a candidate-level review."""
    refs: list[dict[str, Any]] = [
        {
            "authority_kind": "rev3",
            "path": EXPECTED_REV3_CENSUS_MEMBER,
            "locator": ["row_ordinal", candidate["source_binding"]["row_ordinal"]],
            "raw_sha256": EXPECTED_REV3_CENSUS_SHA256,
        }
    ]
    catalog_raw = (
        ROOT / "sources/m2_5/closures/B2/requirement_family_catalog.v1.json"
    ).read_bytes()
    family_ids = {
        ref["semantic_ref"]
        for ref in candidate["participant_refs"]
        if ref["participant_kind"] == "requirement_family"
    }
    for family_id in sorted(family_ids, key=lambda value: canonical(value)):
        if family_id not in {
            item["family_id"] for item in list_value(catalog.get("families"), "B2 families")
        }:
            fail("FAMILY_UNKNOWN", f"candidate evidence references {family_id}")
        refs.append(
            {
                "authority_kind": "b2",
                "path": "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
                "locator": ["family_id", family_id],
                "raw_sha256": sha256_bytes(catalog_raw),
            }
        )
    ordered = sorted(
        refs, key=lambda item: canonical(evidence_ref_cbor(item, "candidate evidence"))
    )
    return evidence_sort(ordered, f"candidate {candidate['candidate_id']} review evidence")


def make_review_domain_assessments(
    candidate: dict[str, Any], evidence_refs: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    """Materialize all eleven candidate-level review-domain assessments."""
    trigger = candidate["relation"] == "declared_card_trigger"
    assessments: list[dict[str, Any]] = []
    for domain in REVIEW_DOMAINS:
        if trigger:
            applicability = "applicable" if domain == "triggers_and_lki" else "not_applicable"
        else:
            applicability = "unresolved"
        assessments.append(
            {
                "review_domain": domain,
                "applicability": applicability,
                "evidence_refs": copy.deepcopy(evidence_refs),
            }
        )
    return assessments


def make_classifications(
    candidates: list[dict[str, Any]],
    classes: list[dict[str, Any]],
    instances: dict[str, dict[str, str]],
    catalog: dict[str, Any],
) -> list[dict[str, Any]]:
    classes_by_osi: dict[str, dict[str, Any]] = {
        record["participant_roles"][0]["semantic_ref"]: record for record in classes
    }
    b2_by_osi = b2_classification_map()
    catalog_by_id, _, _ = b2_family_maps(
        mapping(
            parse_json(
                (ROOT / "sources/m2_5/closures/B2/requirement_family_catalog.v1.json").read_bytes(),
                "B2 catalog",
            ),
            "B2 catalog",
        )
    )
    result: list[dict[str, Any]] = []
    for candidate in candidates:
        cid = candidate["candidate_id"]
        instance_id = instances[cid]["source_instance_id"]
        evidence_refs = candidate_review_evidence(candidate, catalog)
        instance = {
            "source_instance_id": instance_id,
            "participant_bindings": [
                {
                    "role": "trigger_source"
                    if candidate["relation"] == "declared_card_trigger"
                    else "ordered_participant",
                    "participant_ref": copy.deepcopy(ref),
                }
                for ref in candidate["participant_refs"]
            ],
            "context_binding": context_not_applicable(),
            "b2_assignment_refs": [],
            "b1_final_citation_refs": [],
        }
        if candidate["relation"] == "declared_card_trigger":
            oid = candidate["participant_refs"][0]["semantic_ref"]
            class_record = classes_by_osi[oid]
            b2_families, _ = b2_refs_for_osi(oid, catalog_by_id, b2_by_osi)
            assignments = list_value(
                b2_by_osi[oid].get("requirement_assignments"), f"B2 assignments {oid}"
            )
            assignment_by_family = {
                assignment["requirement_family_id"]: index
                for index, assignment in enumerate(assignments)
            }
            b2_assignment_refs: list[dict[str, Any]] = [
                {
                    "family_id": family["family_id"],
                    "assignment_ordinal": assignment_by_family[family["family_id"]],
                    "precise_semantic_definition": next(
                        ref["precise_semantic_definition"]
                        for ref in class_record["b2_boundary_refs"]
                        if ref["family_id"] == family["family_id"]
                    ),
                }
                for family in b2_families
            ]

            def assignment_sort_key(item: dict[str, Any]) -> bytes:
                return canonical(
                    [
                        item["family_id"],
                        item["assignment_ordinal"],
                        item["precise_semantic_definition"],
                    ]
                )

            b2_assignment_refs.sort(key=cast(Callable[[dict[str, Any]], Any], assignment_sort_key))
            instance["b2_assignment_refs"] = b2_assignment_refs
            instance["b1_final_citation_refs"] = copy.deepcopy(
                class_record["b1_final_citation_refs"]
            )
            evidence_refs = copy.deepcopy(class_record["source_evidence_refs"])
            review_state = "resolved"
            disposition = "required_interaction"
            class_id: str | None = class_record["interaction_class_id"]
            rationale = (
                f"Candidate {cid} is the pinned REV3 DECLARED_CARD_TRIGGER row at "
                f"ordinal {candidate['source_binding']['row_ordinal']} with OSI "
                f"{oid}. The unique REV3 resolution join, terminal B2 assignment "
                "evidence, and accepted CR-603 citation establish the required "
                "unary declared interaction within the finite C model."
            )
            unresolved_reason: str | None = None
        else:
            left = candidate["participant_refs"][0]["semantic_ref"]
            right = candidate["participant_refs"][1]["semantic_ref"]
            review_state = "unresolved"
            disposition = None
            class_id = None
            unresolved_reason = "insufficient_pair_relation_authority"
            catalog_by_id, _, _ = b2_family_maps(catalog)
            left_boundary = catalog_by_id[left]["precise_semantic_definition"]
            right_boundary = catalog_by_id[right]["precise_semantic_definition"]
            rationale = (
                f"Candidate {cid} reviews the concrete requirement-family "
                f"participants {left} and {right} from REV3 row ordinal "
                f"{candidate['source_binding']['row_ordinal']}. The available B2 "
                f"boundaries are {left!r}: {left_boundary!r} and {right!r}: "
                f"{right_boundary!r}. The pinned authorities provide no "
                "candidate-specific Pair/Relation Review Authority that can "
                "prove an interaction or positively disprove the relation; this "
                "candidate therefore remains unresolved. Absence of interaction "
                "evidence is not treated as a non-interaction proof."
            )
        assessments = make_review_domain_assessments(candidate, evidence_refs)
        result.append(
            {
                "candidate_id": cid,
                "review_state": review_state,
                "review_domain_assessments": assessments,
                "terminal_disposition": disposition,
                "interaction_class_id": class_id,
                "unresolved_reason": unresolved_reason,
                "source_instance_context_mappings": [instance],
                "reconciliation": {
                    "status": candidate["reconciliation_status"],
                    "original_rev3_candidate_id": cid,
                    "linkage": None,
                },
                "review_rationale": rationale,
                "evidence_refs": evidence_refs,
            }
        )
    return result


def migration_source_classifications() -> tuple[bytes, list[dict[str, Any]]]:
    """Read the superseded V2 monolith only for the one-time V3 migration."""
    raw = git_bytes(
        [
            "show",
            f"{MIGRATION_SOURCE_EXECUTION_COMMIT}:{MIGRATION_SOURCE_CLASSIFICATIONS_PATH}",
        ]
    )
    value = mapping(parse_json(raw, MIGRATION_SOURCE_CLASSIFICATIONS_PATH), "V2 classifications")
    exact_keys(
        value,
        {
            "schema",
            "model_id",
            "candidate_universe_raw_sha256",
            "semantic_classes_raw_sha256",
            "classification_count",
            "candidate_classifications",
        },
        "V2 classifications",
    )
    require(
        value["schema"] == "manafold.m2.5.c.interaction-classifications.v2",
        "MIGRATION_SOURCE_INVALID",
        "V2 migration source schema",
    )
    records = [mapping(item, "V2 classification record") for item in list_value(
        value["candidate_classifications"], "V2 classification records"
    )]
    require(
        value["classification_count"] == len(records) == EXPECTED_REV3_CANDIDATE_COUNT,
        "MIGRATION_SOURCE_INVALID",
        "V2 migration source record count",
    )
    return raw, records


def candidate_ordered_records(records: list[dict[str, Any]]) -> list[dict[str, Any]]:
    seen: set[str] = set()
    for index, record in enumerate(records):
        candidate_id = nonempty_string(
            record.get("candidate_id"), f"classification {index}.candidate_id"
        )
        if candidate_id in seen:
            fail("MIGRATION_SOURCE_INVALID", f"duplicate V2 classification {candidate_id}")
        seen.add(candidate_id)
    return sorted(records, key=lambda record: canonical(record["candidate_id"]))


def make_classification_bundle(
    records: list[dict[str, Any]], universe_raw: bytes, classes_raw: bytes
) -> tuple[dict[str, Any], dict[str, bytes], list[dict[str, Any]]]:
    """Build the V3 root and exact fixed-size shards without changing records."""
    ordered = candidate_ordered_records(records)
    require(
        len(ordered) == EXPECTED_REV3_CANDIDATE_COUNT,
        "REV3_CANDIDATE_COUNT_MISMATCH",
        "V3 classification source count",
    )
    universe_sha = sha256_bytes(universe_raw)
    classes_sha = sha256_bytes(classes_raw)
    shard_bytes: dict[str, bytes] = {}
    shard_entries: list[dict[str, Any]] = []
    for index in range(SHARD_COUNT):
        start = 1000 * index
        end = min(1000 * (index + 1), len(ordered))
        chunk = ordered[start:end]
        require(bool(chunk), "CLASSIFICATION_PARTITION_INVALID", f"empty shard {index}")
        relative = SHARD_NAMES[index]
        shard = {
            "schema": C_JSON_SCHEMAS[SHARD_SCHEMA_NAME],
            "model_id": MODEL_ID,
            "candidate_universe_raw_sha256": universe_sha,
            "semantic_classes_raw_sha256": classes_sha,
            "partition_scheme": PARTITION_SCHEME,
            "shard_index": index,
            "shard_count": SHARD_COUNT,
            "ordinal_start": start,
            "ordinal_end_exclusive": end,
            "record_count": len(chunk),
            "first_candidate_id": chunk[0]["candidate_id"],
            "last_candidate_id": chunk[-1]["candidate_id"],
            "candidate_classifications": copy.deepcopy(chunk),
        }
        raw = json_bytes(shard)
        validate_shard_size(len(raw), index)
        shard_bytes[relative] = raw
        shard_entries.append(
            {
                "shard_index": index,
                "path": relative,
                "ordinal_start": start,
                "ordinal_end_exclusive": end,
                "record_count": len(chunk),
                "first_candidate_id": chunk[0]["candidate_id"],
                "last_candidate_id": chunk[-1]["candidate_id"],
                "raw_sha256": sha256_bytes(raw),
            }
        )
    root = {
        "schema": C_JSON_SCHEMAS[CLASSIFICATIONS_NAME],
        "model_id": MODEL_ID,
        "candidate_universe_raw_sha256": universe_sha,
        "semantic_classes_raw_sha256": classes_sha,
        "partition_scheme": PARTITION_SCHEME,
        "classification_count": len(ordered),
        "shard_count": SHARD_COUNT,
        "shards": shard_entries,
    }
    return root, shard_bytes, ordered


def migration_parity_evidence(
    source_raw: bytes,
    source_records: list[dict[str, Any]],
    target_records: list[dict[str, Any]],
    upstream_source_digests: dict[str, str],
) -> dict[str, Any]:
    """Prove that V3 changes only V2 classification ordering and containment."""
    source_ordered = candidate_ordered_records(source_records)
    require(
        source_ordered == target_records,
        "V2_V3_CLASSIFICATION_PARITY_MISMATCH",
        "V2 and V3 classification records differ beyond the specified ordering",
    )
    require(
        len(source_ordered) == len(target_records) == EXPECTED_REV3_CANDIDATE_COUNT,
        "V2_V3_CLASSIFICATION_PARITY_MISMATCH",
        "migration record counts differ",
    )
    return {
        "source_execution_commit": MIGRATION_SOURCE_EXECUTION_COMMIT,
        "source_classifications_path": MIGRATION_SOURCE_CLASSIFICATIONS_PATH,
        "source_classifications_raw_sha256": sha256_bytes(source_raw),
        "source_upstream_artifact_digests": dict(sorted(upstream_source_digests.items())),
        "target_classification_root_path": CLASSIFICATIONS_NAME,
        "target_shard_paths": list(SHARD_NAMES),
        "source_record_count": len(source_records),
        "target_record_count": len(target_records),
        "record_equality": "PASS",
        "ordering_transform": PARTITION_SCHEME,
        "result": "PASS",
    }


def validate_migration_upstream_parity(name: str, source_raw: bytes, current_raw: bytes) -> None:
    require(
        current_raw == source_raw,
        "V2_V3_UPSTREAM_PARITY_MISMATCH",
        f"{name} differs from the archived V2 H_exec source",
    )


def validate_shard_size(size: int, index: int) -> None:
    require(
        size <= SHARD_MAX_BYTES,
        "CLASSIFICATION_SHARD_SIZE_LIMIT",
        f"classification shard {index} is {size} bytes",
    )


def make_closure(
    model_raw: bytes,
    review_raw: bytes,
    universe_raw: bytes,
    classes_raw: bytes,
    classifications_raw: bytes,
    catalog: dict[str, Any],
    candidates: list[dict[str, Any]],
    classes: list[dict[str, Any]],
    classifications: list[dict[str, Any]],
    reader: ArchiveReader,
) -> dict[str, Any]:
    class_shapes = Counter(record["arity"] for record in classes)
    direction_shapes = Counter(record["directionality"] for record in classes)
    resolved = Counter(
        record["terminal_disposition"]
        for record in classifications
        if record["review_state"] == "resolved"
    )
    unresolved = sum(record["review_state"] == "unresolved" for record in classifications)
    return {
        "schema": C_JSON_SCHEMAS[CLOSURE_NAME],
        "model_id": MODEL_ID,
        "bound_semantic_inputs": [
            {
                "path": "sources/m2_5/closures/C/declared_interaction_model.v2.json",
                "schema": C_JSON_SCHEMAS[MODEL_NAME],
                "raw_sha256": sha256_bytes(model_raw),
                "record_count": 1,
            },
            {
                "path": "sources/m2_5/closures/C/interaction_review_additions.v2.json",
                "schema": C_JSON_SCHEMAS[REVIEW_NAME],
                "raw_sha256": sha256_bytes(review_raw),
                "record_count": 0,
            },
            {
                "path": "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
                "schema": C_JSON_SCHEMAS[UNIVERSE_NAME],
                "raw_sha256": sha256_bytes(universe_raw),
                "record_count": len(candidates),
            },
            {
                "path": "sources/m2_5/closures/C/interaction_semantic_classes.v2.json",
                "schema": C_JSON_SCHEMAS[CLASSES_NAME],
                "raw_sha256": sha256_bytes(classes_raw),
                "record_count": len(classes),
            },
            {
                "path": "sources/m2_5/closures/C/interaction_classifications.v3.json",
                "schema": C_JSON_SCHEMAS[CLASSIFICATIONS_NAME],
                "raw_sha256": sha256_bytes(classifications_raw),
                "record_count": len(classifications),
            },
        ],
        "external_prerequisite_identities": {
            "rev3_archive": {
                "archive_member": EXPECTED_REV3_CENSUS_MEMBER,
                "archive_member_sha256": reader.entry_sha[EXPECTED_REV3_CENSUS_MEMBER],
                "source_package_sha256": EXPECTED_ARCHIVE_SHA256,
            },
            "b2": b2_file_records(),
            "b1_final": b1_final_file_records(),
        },
        "candidate_reconciliation": {
            "rev3_total": len(candidates),
            "rev3_unchanged": len(candidates),
            "rev3_stale": 0,
            "rev3_removed_not_interaction": 0,
            "rev3_merged_semantic_duplicate": 0,
            "new_b2_derived": 0,
            "new_targeted_higher_order": 0,
            "current_total": len(candidates),
        },
        "semantic_class_metrics": {
            "class_count": len(classes),
            "arity_counts": dict(sorted(class_shapes.items())),
            "directionality_counts": dict(sorted(direction_shapes.items())),
            "higher_order_participant_count_rule": "len(participant_roles)",
        },
        "review_state_metrics": {
            "resolved_required_interaction": resolved.get("required_interaction", 0),
            "resolved_not_an_interaction_with_proof": resolved.get(
                "not_an_interaction_with_proof", 0
            ),
            "resolved_out_of_declared_scope_with_reason": resolved.get(
                "out_of_declared_scope_with_reason", 0
            ),
            "unresolved": unresolved,
        },
        "source_instance_metrics": {
            "source_instance_count": len(candidates),
            "candidate_count": len(candidates),
            "duplicate_canonical_source_instance_tuples": 0,
        },
        "gate_status": dict(EXPECTED_GATE_STATUS),
        "flags": dict(EXPECTED_FLAGS),
    }


def make_report(
    model: dict[str, Any],
    review: dict[str, Any],
    universe: dict[str, Any],
    classes: dict[str, Any],
    classifications: dict[str, Any],
    closure: dict[str, Any],
    matrix: dict[str, Any],
    migration: dict[str, Any] | None = None,
    publishability: dict[str, Any] | None = None,
) -> str:
    metrics = closure["review_state_metrics"]
    closure_sha = sha256_bytes(json_bytes(closure))
    domain_counts: dict[str, Counter[str]] = {domain: Counter() for domain in REVIEW_DOMAINS}
    for record in classifications["candidate_classifications"]:
        for assessment in record["review_domain_assessments"]:
            domain_counts[assessment["review_domain"]][assessment["applicability"]] += 1
    b2_bindings = ", ".join(
        f"{item['path']}={item['raw_sha256']}"
        for item in closure["external_prerequisite_identities"]["b2"]
    )
    b1_final_bindings = ", ".join(
        f"{item['path']}={item['raw_sha256']}"
        for item in closure["external_prerequisite_identities"]["b1_final"]
    )
    return "\n".join(
        [
            "# M2.5.C Interaction Model Report",
            "",
            "Status: source snapshot generated for C verification; this report "
            "is derived documentation, not a closure input.",
            "",
            "## Identity and authority",
            "",
            f"- REV3 model: `{EXPECTED_REV3_MODEL_ID}`; "
            f"candidate member: `{EXPECTED_REV3_CENSUS_MEMBER}`.",
            f"- REV3 archive SHA-256: `{EXPECTED_ARCHIVE_SHA256}`.",
            f"- Verified implementation base/master: `{EXPECTED_PREVIOUS_MASTER}`; "
            "the exact H_exec is recorded only in the verification summary.",
            "- Authority graph: model -> review additions -> candidate universe -> "
            "semantic classes -> classifications -> closure -> report/evidence.",
            "- Digest graph: model + review additions + candidate universe + "
            "semantic classes + classification root (which binds all shards) "
            "-> closure; report, negative "
            "matrix, and verification summary are outside the closure.",
            "- The closure binds exactly five semantic C inputs and does not bind "
            "this report, the negative matrix, or the verification summary.",
            f"- Closure status: `{closure['gate_status']['DECLARED_INTERACTION_MODEL_CLOSURE']}`; "
            f"raw SHA-256: `{closure_sha}`.",
            f"- Fixed classification layout: `16` shards with `15 x 1000 + 679` records; "
            f"root `{CLASSIFICATIONS_NAME}` binds every shard.",
            f"- B2 binding summary: `{b2_bindings}`.",
            f"- B1.Final binding summary: `{b1_final_bindings}`.",
            "",
            "## Reconciliation",
            "",
            f"- REV3 candidates: `{closure['candidate_reconciliation']['rev3_total']}`; "
            f"current total: `{closure['candidate_reconciliation']['current_total']}`.",
            f"- Unchanged: `{closure['candidate_reconciliation']['rev3_unchanged']}`; "
            "stale/removed/merged: `0/0/0`.",
            "- New B2-derived candidates: `0` (forbidden in C V3); "
            "targeted higher-order additions: `0`.",
            "",
            "## Review state",
            "",
            f"- Resolved required interaction: `{metrics['resolved_required_interaction']}`.",
            "- Resolved not an interaction with proof: "
            f"`{metrics['resolved_not_an_interaction_with_proof']}`.",
            "- Resolved out of declared scope: "
            f"`{metrics['resolved_out_of_declared_scope_with_reason']}`.",
            f"- Unresolved: `{metrics['unresolved']}`.",
            f"- Semantic classes: `{classes['class_count']}`; "
            f"source instances: `{universe['source_instance_count']}`.",
            f"- Review additions: `{review['review_record_count']}`; "
            "targeted higher-order authority is empty in C V3.",
            f"- V2 to V3 migration parity: `{'PASS' if migration is not None else 'NOT_RUN'}`; "
            f"source `{MIGRATION_SOURCE_EXECUTION_COMMIT}`; target `{CLASSIFICATIONS_NAME}`.",
            f"- Git-history publishability preflight: `"
            f"{(publishability or {}).get('result', 'NOT_RUN')}`; "
            f"checked commit `"
            f"{(publishability or {}).get('checked_commit', 'H_exec not created')}`; "
            f"hosting limit `{HOSTING_HARD_LIMIT_BYTES}` bytes.",
            "",
            "## Required review-domain coverage",
            "",
            *[
                f"- {domain}: applicable={counts.get('applicable', 0)}, "
                f"not_applicable={counts.get('not_applicable', 0)}, "
                f"unresolved={counts.get('unresolved', 0)}."
                for domain, counts in domain_counts.items()
            ],
            "",
            "## Evidence boundary",
            "",
            "Card-trigger classes use their exact joined OSI, terminal B2 "
            "assignments/boundaries, and CR-603 B1.Final citation. Family-pair "
            "rows remain concrete source instances and are not promoted from "
            "co-occurrence alone.",
            "- High-risk review coverage is derived from the authoritative eleven "
            "candidate-level assessments; family-pair rows remain unresolved "
            "until a separately approved Pair/Relation Review Authority exists.",
            "",
            "## Gate state",
            "",
            "```text",
            *[f"{key} = {value}" for key, value in closure["gate_status"].items()],
            *[f"{key} = {str(value).lower()}" for key, value in closure["flags"].items()],
            "```",
            "",
            "## Verification matrix",
            "",
            f"- Negative cases: `{len(matrix['cases'])}` (C-001 through C-042).",
            "",
            "## Phase B command status at H_exec creation",
            "",
            "These commands were not yet run against the exact H_exec snapshot; "
            "their actual Phase B results are recorded in the final summary.",
            "- `py -3.13 scripts/check_m2_5_master_drift.py`: `NOT_RUN`.",
            "- `py -3.13 scripts/check_m2_5_master_drift.py --negative-self-test`: `NOT_RUN`.",
            "- `py -3.13 scripts/check_m2_5_master_drift.py --verify-archive`: `NOT_RUN`.",
            "- `py -3.13 scripts/check_m2_5_b1_authority_citations.py`: `NOT_RUN`.",
            "- `py -3.13 scripts/check_m2_5_b1_authority_citations.py "
            "--negative-self-test`: `NOT_RUN`.",
            "- `py -3.13 scripts/check_m2_5_b2_classifications.py`: `NOT_RUN`.",
            "- `py -3.13 scripts/check_m2_5_b2_classifications.py "
            "--negative-self-test`: `NOT_RUN`.",
            "- `py -3.13 scripts/check_m2_5_b1_final_authority_citations.py`: `NOT_RUN`.",
            "- `py -3.13 scripts/check_m2_5_b1_final_authority_citations.py "
            "--negative-self-test`: `NOT_RUN`.",
            "- `py -3.13 scripts/check_m2_5_c_interactions.py`: `NOT_RUN`.",
            "- `py -3.13 scripts/check_m2_5_c_interactions.py --negative-self-test`: `NOT_RUN`.",
            "- `py -3.13 scripts/verify_repository.py`: `NOT_RUN`.",
            "- `py -3.13 scripts/run_checks.py integration`: `NOT_RUN`.",
            "- `cargo +1.85.1 fmt --all -- --check`: `NOT_RUN`.",
            "- `cargo +1.85.1 check --workspace --all-targets --all-features --locked`: `NOT_RUN`.",
            "- Applicable Ruff, Mypy, Clippy, Rust tests, schema, conformance, "
            "information-safety, replay, and maintainer gates: `NOT_RUN`.",
            "- Final command statuses are evidence in the verification summary; "
            "the summary remains outside the closure.",
            "",
        ]
    )


def negative_matrix() -> dict[str, Any]:
    rows = [
        (
            "C-001",
            "Make a required prerequisite gate non-terminal",
            CLOSURE_NAME,
            "BLOCKED",
            "PREREQUISITE_NOT_PASS",
        ),
        (
            "C-002",
            "Change the B2 catalog identity",
            UNIVERSE_NAME,
            "BLOCKED",
            "B2_CATALOG_DIGEST_MISMATCH",
        ),
        (
            "C-003",
            "Change the B2 classification identity",
            UNIVERSE_NAME,
            "BLOCKED",
            "B2_CLASSIFICATIONS_DIGEST_MISMATCH",
        ),
        (
            "C-004",
            "Change a B2 boundary binding",
            CLASSES_NAME,
            "FAIL",
            "B2_BOUNDARY_BINDING_MISMATCH",
        ),
        (
            "C-005",
            "Change the B1.Final citation-graph identity",
            CLOSURE_NAME,
            "BLOCKED",
            "B1_FINAL_GRAPH_DIGEST_MISMATCH",
        ),
        (
            "C-006",
            "Change the pinned REV3 archive digest",
            CLOSURE_NAME,
            "BLOCKED",
            "REV3_ARCHIVE_DIGEST_MISMATCH",
        ),
        (
            "C-007",
            "Remove one inherited REV3 candidate",
            UNIVERSE_NAME,
            "FAIL",
            "REV3_CANDIDATE_UNACCOUNTED",
        ),
        (
            "C-008",
            "Leave a candidate unresolved while claiming closure PASS",
            CLASSIFICATIONS_NAME,
            "FAIL",
            "UNRESOLVED_CANDIDATE_ON_PASS",
        ),
        ("C-009", "Reference an unknown OSI", UNIVERSE_NAME, "FAIL", "OSI_UNKNOWN"),
        ("C-010", "Reference an unknown B2 family", UNIVERSE_NAME, "FAIL", "FAMILY_UNKNOWN"),
        (
            "C-011",
            "Reference an invalid assignment",
            CLASSIFICATIONS_NAME,
            "FAIL",
            "ASSIGNMENT_BINDING_INVALID",
        ),
        (
            "C-012",
            "Use ACTIVE_UNASSIGNED as card-derived proof",
            CLASSES_NAME,
            "FAIL",
            "ACTIVE_UNASSIGNED_CARD_DERIVED",
        ),
        (
            "C-013",
            "Duplicate an interaction class ID with different meaning",
            CLASSES_NAME,
            "FAIL",
            "DUPLICATE_CLASS_ID",
        ),
        (
            "C-014",
            "Duplicate a candidate/source-instance mapping",
            CLASSIFICATIONS_NAME,
            "FAIL",
            "DUPLICATE_SOURCE_INSTANCE_MAPPING",
        ),
        (
            "C-015",
            "Add a source instance with no candidate owner",
            UNIVERSE_NAME,
            "FAIL",
            "ORPHAN_SOURCE_INSTANCE",
        ),
        ("C-016", "Reverse a directed relation", UNIVERSE_NAME, "FAIL", "DIRECTION_REVERSED"),
        (
            "C-017",
            "Remove the direction from a directed relation",
            UNIVERSE_NAME,
            "FAIL",
            "DIRECTIONALITY_LOST",
        ),
        (
            "C-018",
            "Remove a required participant role",
            CLASSES_NAME,
            "FAIL",
            "PARTICIPANT_ROLE_MISSING",
        ),
        (
            "C-019",
            "Remove one participant from a higher-order class",
            CLASSES_NAME,
            "FAIL",
            "HIGHER_ORDER_PARTICIPANT_MISSING",
        ),
        (
            "C-020",
            "Rewrite same-host context as cross-host",
            CLASSES_NAME,
            "FAIL",
            "HOST_RELATIONSHIP_MISMATCH",
        ),
        (
            "C-021",
            "Rewrite cross-host context as same-host",
            CLASSES_NAME,
            "FAIL",
            "HOST_RELATIONSHIP_MISMATCH",
        ),
        (
            "C-022",
            "Remove a required context dimension",
            CLASSES_NAME,
            "FAIL",
            "CONTEXT_DIMENSION_MISSING",
        ),
        (
            "C-023",
            "Remove a required B1.Final citation reference",
            CLASSES_NAME,
            "FAIL",
            "B1_CITATION_UNRESOLVED",
        ),
        (
            "C-024",
            "Add an authority not present in accepted inputs",
            CLASSES_NAME,
            "FAIL",
            "UNAPPROVED_AUTHORITY",
        ),
        (
            "C-025",
            "Bind C to a stale but internally self-consistent prerequisite",
            CLOSURE_NAME,
            "BLOCKED",
            "PREREQUISITE_IDENTITY_STALE",
        ),
        (
            "C-026",
            "Tamper with an aggregate count",
            CLOSURE_NAME,
            "FAIL",
            "AGGREGATE_COUNT_MISMATCH",
        ),
        (
            "C-027",
            "Use a non-terminal disposition in a PASS closure",
            CLASSIFICATIONS_NAME,
            "FAIL",
            "NONTERMINAL_DISPOSITION_ON_PASS",
        ),
        (
            "C-028",
            "Promote ranking/reuse status",
            CLOSURE_NAME,
            "FAIL",
            "DOWNSTREAM_STATUS_PROMOTED",
        ),
        ("C-029", "Promote deck-lock status", CLOSURE_NAME, "FAIL", "DOWNSTREAM_STATUS_PROMOTED"),
        ("C-030", "Promote M3 status", CLOSURE_NAME, "FAIL", "DOWNSTREAM_STATUS_PROMOTED"),
        (
            "C-031",
            "Change a source artifact after H_exec",
            SUMMARY_NAME,
            "FAIL",
            "SOURCE_CHANGED_AFTER_H_EXEC",
        ),
        (
            "C-032",
            "Change the evidence summary's recorded artifact digest",
            SUMMARY_NAME,
            "FAIL",
            "EVIDENCE_DIGEST_BINDING_MISMATCH",
        ),
        (
            "C-033",
            "Replace a normalized semantic enum with an uppercase/noncanonical variant",
            UNIVERSE_NAME,
            "FAIL",
            "NONCANONICAL_ENUM_VARIANT",
        ),
        (
            "C-034",
            "Add an unknown participant/context/temporal vocabulary variant",
            MODEL_NAME,
            "FAIL",
            "VOCABULARY_VARIANT_UNKNOWN",
        ),
        (
            "C-035",
            "Remove the targeted review record named by a candidate",
            UNIVERSE_NAME,
            "FAIL",
            "TARGETED_REVIEW_RECORD_MISSING",
        ),
        (
            "C-036",
            "Reference an unknown targeted review record",
            UNIVERSE_NAME,
            "FAIL",
            "TARGETED_REVIEW_RECORD_UNKNOWN",
        ),
        (
            "C-037",
            "Tamper with the review-additions raw binding",
            UNIVERSE_NAME,
            "FAIL",
            "REVIEW_ADDITIONS_DIGEST_MISMATCH",
        ),
        (
            "C-038",
            "Inject a b2_derived candidate into a valid V3 snapshot",
            UNIVERSE_NAME,
            "FAIL",
            "B2_DERIVED_FORBIDDEN_V1",
        ),
        (
            "C-039",
            "Tamper with a CandidateIdentityV1 preimage or digest",
            UNIVERSE_NAME,
            "FAIL",
            "CANDIDATE_IDENTITY_MISMATCH",
        ),
        (
            "C-040",
            "Tamper with a recorded C or master-drift checker identity",
            SUMMARY_NAME,
            "FAIL",
            "CHECKER_IDENTITY_MISMATCH",
        ),
        (
            "C-041",
            "Duplicate a canonical source-instance tuple within one candidate",
            UNIVERSE_NAME,
            "FAIL",
            "DUPLICATE_SOURCE_INSTANCE_TUPLE",
        ),
        (
            "C-042",
            "Replace archive_member_sha256 with a 63-character lowercase hexadecimal value",
            UNIVERSE_NAME,
            "FAIL",
            "SHA256_SCALAR_ENCODING_INVALID",
        ),
    ]
    return {
        "schema": C_JSON_SCHEMAS[MATRIX_NAME],
        "model_id": MODEL_ID,
        "cases": [
            {
                "case_id": case_id,
                "mutation": mutation,
                "expected_status": status,
                "expected_reason_code": code,
                "target_artifact": target,
            }
            for case_id, mutation, target, status, code in rows
        ],
    }


def make_summary(status: str = "NOT_RUN") -> dict[str, Any]:
    return {
        "schema": C_JSON_SCHEMAS[SUMMARY_NAME],
        "execution_commit": None,
        "source_tree_before_fingerprint": None,
        "source_tree_after_fingerprint": None,
        "prerequisite_results": {"status": status, "commands": []},
        "c_result": {"status": status, "reason": "H_exec source snapshot is provisional."},
        "negative_test_result": {"status": status, "case_count": 42},
        "repository_gate_results": {"status": status, "commands": []},
        "artifact_digests": {},
        "migration_parity": {
            "source_execution_commit": MIGRATION_SOURCE_EXECUTION_COMMIT,
            "source_classifications_path": MIGRATION_SOURCE_CLASSIFICATIONS_PATH,
            "source_classifications_raw_sha256": "0" * 64,
            "source_upstream_artifact_digests": {},
            "target_classification_root_path": CLASSIFICATIONS_NAME,
            "target_shard_paths": list(SHARD_NAMES),
            "source_record_count": 0,
            "target_record_count": 0,
            "record_equality": status,
            "ordering_transform": PARTITION_SCHEME,
            "result": status,
        },
        "publishability_preflight": {
            "base_ref": "origin/master",
            "verified_base_commit": EXPECTED_PREVIOUS_MASTER,
            "checked_commit": None,
            "introduced_object_count": 0,
            "introduced_blob_count": 0,
            "hosting_hard_limit_bytes": HOSTING_HARD_LIMIT_BYTES,
            "oversized_blobs": [],
            "forbidden_legacy_paths": [],
            "result": status,
        },
        "checker_identities": {
            "c_checker": {"path": CHECKER_RELATIVE_PATH, "raw_sha256": "0" * 64},
            "master_drift_checker": {
                "path": "scripts/check_m2_5_master_drift.py",
                "raw_sha256": "0" * 64,
            },
        },
        "evidence_protocol": {
            "H_exec": None,
            "modified_path": "sources/m2_5/closures/C/verification/c_verification_summary.v3.json",
            "H_evidence_relation": "direct_child_summary_only",
        },
        "evidence_export": {"status": status, "path": None, "sha256": None},
    }


def generate_artifacts() -> None:
    """Generate the publishable V3 bundle from reviewed V2 source bytes."""
    reader = load_archive()
    upstream_names = (MODEL_NAME, REVIEW_NAME, UNIVERSE_NAME, CLASSES_NAME)
    upstream_raw: dict[str, bytes] = {}
    for name in upstream_names:
        relative = f"sources/m2_5/closures/C/{name}"
        source_raw = git_bytes(["show", f"{MIGRATION_SOURCE_EXECUTION_COMMIT}:{relative}"])
        current_raw = local_raw(name)
        validate_migration_upstream_parity(name, source_raw, current_raw)
        upstream_raw[name] = current_raw
    model_raw = upstream_raw[MODEL_NAME]
    review_raw = upstream_raw[REVIEW_NAME]
    universe_raw = upstream_raw[UNIVERSE_NAME]
    classes_raw = upstream_raw[CLASSES_NAME]
    model = mapping(parse_json(model_raw, MODEL_NAME), MODEL_NAME)
    review = mapping(parse_json(review_raw, REVIEW_NAME), REVIEW_NAME)
    universe = mapping(parse_json(universe_raw, UNIVERSE_NAME), UNIVERSE_NAME)
    classes = mapping(parse_json(classes_raw, CLASSES_NAME), CLASSES_NAME)
    catalog = mapping(
        parse_json(
            (ROOT / "sources/m2_5/closures/B2/requirement_family_catalog.v1.json").read_bytes(),
            "B2 catalog",
        ),
        "B2 catalog",
    )
    candidates, _instances = expected_candidates(reader, catalog)
    source_classifications_raw, source_classifications = migration_source_classifications()
    classification_root, shard_bytes, classifications_list = make_classification_bundle(
        source_classifications, universe_raw, classes_raw
    )
    migration_upstream_digests = {
        f"sources/m2_5/closures/C/{name}": sha256_bytes(
            git_bytes(
                [
                    "show",
                    f"{MIGRATION_SOURCE_EXECUTION_COMMIT}:sources/m2_5/closures/C/{name}",
                ]
            )
        )
        for name in upstream_names
    }
    migration = migration_parity_evidence(
        source_classifications_raw,
        source_classifications,
        classifications_list,
        migration_upstream_digests,
    )
    classifications_raw = json_bytes(classification_root)
    classification_view = {
        **classification_root,
        "candidate_classifications": classifications_list,
    }
    closure = make_closure(
        model_raw,
        review_raw,
        universe_raw,
        classes_raw,
        classifications_raw,
        catalog,
        candidates,
        list_value(classes["classes"], "semantic class records"),
        classifications_list,
        reader,
    )
    closure_raw = json_bytes(closure)
    matrix = negative_matrix()
    matrix_raw = json_bytes(matrix)
    report = make_report(
        model,
        review,
        universe,
        classes,
        classification_view,
        closure,
        matrix,
        migration,
    ).encode("utf-8")
    summary = make_summary()
    summary["migration_parity"] = migration
    summary["artifact_digests"] = {
        "C_DESIGN_SPEC.md": sha256_bytes(local_raw("C_DESIGN_SPEC.md")),
        MODEL_NAME: sha256_bytes(model_raw),
        REVIEW_NAME: sha256_bytes(review_raw),
        UNIVERSE_NAME: sha256_bytes(universe_raw),
        CLASSES_NAME: sha256_bytes(classes_raw),
        CLASSIFICATIONS_NAME: sha256_bytes(classifications_raw),
        **{name: sha256_bytes(raw) for name, raw in shard_bytes.items()},
        CLOSURE_NAME: sha256_bytes(closure_raw),
        REPORT_NAME: sha256_bytes(report),
        MATRIX_NAME: sha256_bytes(matrix_raw),
    }
    summary["checker_identities"]["c_checker"]["raw_sha256"] = sha256_bytes(
        Path(__file__).read_bytes()
    )
    drift = ROOT / "scripts/check_m2_5_master_drift.py"
    if drift.is_file():
        summary["checker_identities"]["master_drift_checker"]["raw_sha256"] = sha256_bytes(
            drift.read_bytes()
        )
    summary_raw = json_bytes(summary)
    VERIFICATION_DIR.mkdir(parents=True, exist_ok=True)
    for name, raw in (
        *upstream_raw.items(),
        (CLASSIFICATIONS_NAME, classifications_raw),
        *shard_bytes.items(),
        (CLOSURE_NAME, closure_raw),
        (REPORT_NAME, report),
        (MATRIX_NAME, matrix_raw),
        (SUMMARY_NAME, summary_raw),
    ):
        path = VERIFICATION_DIR / name if name in {MATRIX_NAME, SUMMARY_NAME} else C_DIR / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
    print(
        f"GENERATED candidates={len(candidates)} classes={len(classes['classes'])} "
        f"classifications={len(classifications_list)} shards={len(shard_bytes)} "
        f"source_instances={len(candidates)}"
    )


def require_local_input_bindings(
    universe: dict[str, Any], model_raw: bytes, review_raw: bytes
) -> None:
    bindings = mapping(universe.get("input_bindings"), "candidate universe.input_bindings")
    declared = mapping(bindings.get("declared_model"), "candidate universe declared_model")
    exact_keys(declared, {"path", "raw_sha256"}, "candidate universe declared_model")
    require(
        declared["path"] == "sources/m2_5/closures/C/declared_interaction_model.v2.json",
        "PREREQUISITE_IDENTITY_STALE",
        "candidate universe model path",
    )
    require(
        declared["raw_sha256"] == sha256_bytes(model_raw),
        "PREREQUISITE_IDENTITY_STALE",
        "candidate universe model digest",
    )
    review = mapping(bindings.get("review_additions"), "candidate universe review_additions")
    exact_keys(review, {"path", "raw_sha256"}, "candidate universe review_additions")
    require(
        review["path"] == "sources/m2_5/closures/C/interaction_review_additions.v2.json",
        "REVIEW_ADDITIONS_DIGEST_MISMATCH",
        "candidate universe review path",
    )
    require(
        review["raw_sha256"] == sha256_bytes(review_raw),
        "REVIEW_ADDITIONS_DIGEST_MISMATCH",
        "candidate universe review digest",
    )


def validate_v2_candidate_state(record: dict[str, Any], label: str) -> None:
    """Validate the state-dependent V2 classification grammar."""
    state = enum_value(record.get("review_state"), REVIEW_STATES, f"{label}.review_state")
    disposition = record.get("terminal_disposition")
    class_id = record.get("interaction_class_id")
    reason = record.get("unresolved_reason")
    if state == "resolved":
        if (
            not isinstance(disposition, str)
            or disposition not in EXTRA_VOCABULARY["terminal_disposition"]
        ):
            fail(
                "NONTERMINAL_DISPOSITION_ON_PASS",
                f"{label} resolved state has no terminal disposition",
            )
        if reason is not None:
            fail(
                "NONTERMINAL_DISPOSITION_ON_PASS",
                f"{label} resolved state has an unresolved reason",
            )
        if disposition == "required_interaction":
            if not isinstance(class_id, str) or not class_id:
                fail("CANDIDATE_CLASS_UNRESOLVED", f"{label} required state has no class ID")
        elif class_id is not None:
            fail("NONTERMINAL_DISPOSITION_ON_PASS", f"{label} non-required state has a class ID")
        return
    if disposition is not None:
        fail(
            "NONTERMINAL_DISPOSITION_ON_PASS",
            f"{label} unresolved state has a terminal disposition",
        )
    if class_id is not None:
        fail("NONTERMINAL_DISPOSITION_ON_PASS", f"{label} unresolved state has a class ID")
    enum_value(reason, UNRESOLVED_REASONS, f"{label}.unresolved_reason")


def validate_model(model: dict[str, Any]) -> None:
    exact_keys(
        model,
        {
            "schema",
            "model_id",
            "model_version",
            "coverage_scope",
            "accepted_rev3_model",
            "accepted_rev3_candidate_source",
            "included_shapes",
            "excluded_claims",
            "terminal_dispositions",
            "review_states",
            "unresolved_reasons",
            "review_domain_vocabulary",
            "review_domain_applicability_vocabulary",
            "context_dimensions",
            "authority_policy",
            "participant_kind_vocabulary",
            "participant_role_vocabulary",
            "context_value_vocabulary",
            "temporal_value_vocabulary",
        },
        "declared model",
    )
    require(
        model["schema"] == C_JSON_SCHEMAS[MODEL_NAME], "SCHEMA_MISMATCH", "declared model schema"
    )
    require(
        model["model_id"] == MODEL_ID,
        "MODEL_ID_MISMATCH",
        "declared model ID",
    )
    require(
        model["coverage_scope"] == "pairwise_plus_review_outliers",
        "MODEL_SCOPE_MISMATCH",
        "coverage scope",
    )
    require(
        model["accepted_rev3_model"] == EXPECTED_REV3_MODEL_ID,
        "MODEL_ID_MISMATCH",
        "accepted REV3 model",
    )
    require(
        model["accepted_rev3_candidate_source"] == EXPECTED_REV3_CENSUS_MEMBER,
        "MODEL_SCOPE_MISMATCH",
        "candidate source",
    )
    require(model["model_version"] == MODEL_VERSION, "MODEL_SCOPE_MISMATCH", "model version")
    require(
        model["context_dimensions"] == list(CONTEXT_KEYS),
        "MODEL_SCOPE_MISMATCH",
        "declared context dimensions",
    )
    require(
        model["included_shapes"]
        == [
            "unary_card_specific_declared_outliers",
            "binary_family_relations",
            "directional_binary_relations",
            "explicit_reviewed_higher_order_interactions",
        ],
        "MODEL_SCOPE_MISMATCH",
        "included interaction shapes",
    )
    require(
        model["excluded_claims"] == ["arbitrary_unbounded_n_way_magic_interaction_completeness"],
        "MODEL_SCOPE_MISMATCH",
        "excluded interaction claims",
    )
    require(
        model["participant_kind_vocabulary"] == list(PARTICIPANT_KINDS),
        "VOCABULARY_VARIANT_UNKNOWN",
        "participant kind vocabulary",
    )
    require(
        model["participant_role_vocabulary"] == list(PARTICIPANT_ROLES),
        "VOCABULARY_VARIANT_UNKNOWN",
        "participant role vocabulary",
    )
    for key, values in CONTEXT_VOCABULARY.items():
        require(
            mapping(model["context_value_vocabulary"], "context values").get(key) == list(values),
            "VOCABULARY_VARIANT_UNKNOWN",
            f"context vocabulary {key}",
        )
    for key, values in TEMPORAL_VOCABULARY.items():
        require(
            mapping(model["temporal_value_vocabulary"], "temporal values").get(key) == list(values),
            "VOCABULARY_VARIANT_UNKNOWN",
            f"temporal vocabulary {key}",
        )
    require(
        model["terminal_dispositions"] == list(EXTRA_VOCABULARY["terminal_disposition"]),
        "VOCABULARY_VARIANT_UNKNOWN",
        "terminal dispositions",
    )
    require(
        model["review_states"] == list(REVIEW_STATES),
        "VOCABULARY_VARIANT_UNKNOWN",
        "review states",
    )
    require(
        model["unresolved_reasons"] == list(UNRESOLVED_REASONS),
        "VOCABULARY_VARIANT_UNKNOWN",
        "unresolved reasons",
    )
    require(
        model["review_domain_vocabulary"] == list(REVIEW_DOMAINS),
        "VOCABULARY_VARIANT_UNKNOWN",
        "review domains",
    )
    require(
        model["review_domain_applicability_vocabulary"] == list(REVIEW_DOMAIN_APPLICABILITY),
        "VOCABULARY_VARIANT_UNKNOWN",
        "review-domain applicability",
    )


def validate_review(review: dict[str, Any], model_raw: bytes) -> None:
    exact_keys(
        review,
        {"schema", "model_id", "input_bindings", "review_record_count", "review_records"},
        "review additions",
    )
    require(
        review["schema"] == C_JSON_SCHEMAS[REVIEW_NAME],
        "SCHEMA_MISMATCH",
        "review additions schema",
    )
    require(
        review["model_id"] == MODEL_ID,
        "MODEL_ID_MISMATCH",
        "review additions model",
    )
    bindings = mapping(review["input_bindings"], "review input_bindings")
    exact_keys(
        bindings,
        {"declared_model_path", "declared_model_raw_sha256", "source_evidence_refs_sorted_array"},
        "review input_bindings",
    )
    require(
        bindings["declared_model_path"]
        == "sources/m2_5/closures/C/declared_interaction_model.v2.json",
        "PREREQUISITE_IDENTITY_STALE",
        "review model path",
    )
    require(
        bindings["declared_model_raw_sha256"] == sha256_bytes(model_raw),
        "PREREQUISITE_IDENTITY_STALE",
        "review model digest",
    )
    review_input_evidence = list_value(
        bindings["source_evidence_refs_sorted_array"], "review evidence"
    )
    for evidence in review_input_evidence:
        if mapping(evidence, "review input evidence").get("authority_kind") not in {
            "rev3",
            "b2",
            "b1_final",
        }:
            fail("UNAPPROVED_AUTHORITY", "review input evidence authority")
    evidence_sort(review_input_evidence, "review evidence")
    records = list_value(review["review_records"], "review records")
    require(
        review["review_record_count"] == len(records),
        "AGGREGATE_COUNT_MISMATCH",
        "review record count",
    )
    seen: set[str] = set()
    for i, record in enumerate(records):
        item = mapping(record, f"review record {i}")
        exact_keys(
            item,
            {
                "review_record_id",
                "review_kind",
                "participant_source_refs",
                "review_evidence_refs",
                "review_rationale",
            },
            f"review record {i}",
        )
        rid = nonempty_string(item["review_record_id"], f"review record {i} ID")
        require(
            REVIEW_ID_RE.fullmatch(rid) is not None,
            "TARGETED_REVIEW_RECORD_UNKNOWN",
            f"review record ID {rid}",
        )
        require(rid not in seen, "TARGETED_REVIEW_RECORD_UNKNOWN", f"duplicate review record {rid}")
        seen.add(rid)
        enum(item["review_kind"], EXTRA_VOCABULARY["review_kind"], f"review record {i}.review_kind")
        record_evidence = list_value(item["review_evidence_refs"], f"review record {i}.evidence")
        for evidence in record_evidence:
            if mapping(evidence, "review record evidence").get("authority_kind") not in {
                "rev3",
                "b2",
                "b1_final",
            }:
                fail("UNAPPROVED_AUTHORITY", f"review record {i} evidence authority")
        evidence_sort(record_evidence, f"review record {i}.evidence")
        nonempty_string(item["review_rationale"], f"review record {i}.rationale")


def validate_external_prereqs(
    reader: ArchiveReader, catalog: dict[str, Any], b2: dict[str, Any], b1: dict[str, Any]
) -> None:
    require(
        catalog.get("schema") == "manafold.m2.5.b2.requirement-family-catalog.v1",
        "PREREQUISITE_IDENTITY_STALE",
        "B2 catalog schema",
    )
    require(
        catalog.get("source_package_sha256") == EXPECTED_ARCHIVE_SHA256,
        "PREREQUISITE_IDENTITY_STALE",
        "B2 package identity",
    )
    require(
        catalog.get("catalog_family_count") == 216
        and len(list_value(catalog.get("families"), "B2 families")) == 216,
        "PREREQUISITE_COUNT_MISMATCH",
        "B2 family count",
    )
    statuses = Counter(
        mapping(x, "B2 family").get("status")
        for x in list_value(catalog.get("families"), "B2 families")
    )
    require(
        statuses.get("ACTIVE", 0) == 210 and statuses.get("ACTIVE_UNASSIGNED", 0) == 6,
        "PREREQUISITE_COUNT_MISMATCH",
        "B2 lifecycle counts",
    )
    classifications = list_value(b2.get("classifications"), "B2 classifications")
    require(
        len(classifications) == 402
        and sum(
            len(list_value(x.get("requirement_assignments"), "B2 assignments"))
            for x in classifications
        )
        == 1883,
        "PREREQUISITE_COUNT_MISMATCH",
        "B2 terminal counts",
    )
    projection = load_csv(
        (ROOT / "sources/m2_5/closures/B2/deck_row_classification_refs.v1.csv").read_bytes(),
        "B2 projection",
    )
    require(len(projection) == 441, "PREREQUISITE_COUNT_MISMATCH", "B2 assignment rows")
    require(
        b1.get("schema") == "manafold.m2.5.b1.official-authority-citations.v3",
        "PREREQUISITE_IDENTITY_STALE",
        "B1.Final schema",
    )
    require(
        len(list_value(b1.get("authorities"), "B1.Final authorities")) == 7,
        "PREREQUISITE_COUNT_MISMATCH",
        "B1.Final authority count",
    )
    closure = mapping(
        parse_json(
            (
                ROOT / "sources/m2_5/closures/B1/official_authority_citation_closure.v2.json"
            ).read_bytes(),
            "B1.Final closure",
        ),
        "B1.Final closure",
    )
    require(
        closure.get("REQUIRED_B2_FAMILY_COUNT") == 210,
        "PREREQUISITE_COUNT_MISMATCH",
        "B1.Final active family count",
    )
    require(
        reader.entry_sha.get(EXPECTED_REV3_MODEL_MEMBER) == EXPECTED_REV3_MODEL_SHA256,
        "REV3_MODEL_DIGEST_MISMATCH",
        "REV3 model member",
    )


def validate_b2_and_b1_bindings(
    value: dict[str, Any], label: str, b2_code: str, b1_code: str
) -> None:
    bindings = mapping(value, label)
    expected_b2 = b2_file_records()
    expected_b1 = b1_final_file_records()
    actual_b2 = list_value(bindings.get("b2_artifacts"), f"{label}.b2_artifacts")
    if actual_b2 != expected_b2:
        for index, expected in enumerate(expected_b2):
            if index >= len(actual_b2) or actual_b2[index] != expected:
                code = (
                    "B2_CATALOG_DIGEST_MISMATCH"
                    if index == 0
                    else "B2_CLASSIFICATIONS_DIGEST_MISMATCH"
                    if index == 1
                    else b2_code
                )
                blocked(
                    code, f"{label}.b2_artifacts[{index}] does not match the accepted B2 identity"
                )
        blocked(b2_code, f"{label}.b2_artifacts is not the accepted B2 identity set")
    actual_b1 = list_value(bindings.get("b1_final_artifacts"), f"{label}.b1_final_artifacts")
    if actual_b1 != expected_b1:
        for index, expected in enumerate(expected_b1):
            if index >= len(actual_b1) or actual_b1[index] != expected:
                blocked(
                    b1_code,
                    f"{label}.b1_final_artifacts[{index}] does not match the "
                    "accepted B1.Final identity",
                )
        blocked(b1_code, f"{label}.b1_final_artifacts is not the accepted B1.Final identity set")


def validate_closure_external_bindings(closure: dict[str, Any]) -> None:
    external = mapping(
        closure["external_prerequisite_identities"], "closure external prerequisite identities"
    )
    rev3 = mapping(external.get("rev3_archive"), "closure REV3 archive identity")
    expected_rev3 = {
        "archive_member": EXPECTED_REV3_CENSUS_MEMBER,
        "archive_member_sha256": EXPECTED_REV3_CENSUS_SHA256,
        "source_package_sha256": EXPECTED_ARCHIVE_SHA256,
    }
    if rev3 != expected_rev3:
        blocked("REV3_ARCHIVE_DIGEST_MISMATCH", "closure REV3 archive identity")
    actual_b2 = list_value(external.get("b2"), "closure B2 identities")
    if actual_b2 != b2_file_records():
        blocked("PREREQUISITE_IDENTITY_STALE", "closure B2 identity set")
    actual_b1 = list_value(external.get("b1_final"), "closure B1.Final identities")
    if actual_b1 != b1_final_file_records():
        blocked("B1_FINAL_GRAPH_DIGEST_MISMATCH", "closure B1.Final identity set")


def validate_closure_early(
    closure: dict[str, Any], snapshot: Snapshot, reader: ArchiveReader
) -> None:
    """Check closure-owned guards before traversing the large candidate ledger."""
    exact_keys(
        closure,
        {
            "schema",
            "model_id",
            "bound_semantic_inputs",
            "external_prerequisite_identities",
            "candidate_reconciliation",
            "semantic_class_metrics",
            "review_state_metrics",
            "source_instance_metrics",
            "gate_status",
            "flags",
        },
        "interaction closure",
    )
    require(closure["schema"] == C_JSON_SCHEMAS[CLOSURE_NAME], "SCHEMA_MISMATCH", "closure schema")
    gate_status = mapping(closure["gate_status"], "closure gate status")
    for gate in (
        "CLASSIFICATION_REFERENCE_CLOSURE",
        "OFFICIAL_RULE_CITATION_CLOSURE",
    ):
        if gate_status.get(gate) != "PASS":
            blocked("PREREQUISITE_NOT_PASS", f"required prerequisite gate {gate} is not PASS")
    if gate_status not in (EXPECTED_GATE_STATUS, EXPECTED_PASS_GATE_STATUS):
        fail("DOWNSTREAM_STATUS_PROMOTED", "closure gate vocabulary/status")
    if mapping(closure["flags"], "closure flags") != EXPECTED_FLAGS:
        fail("DOWNSTREAM_STATUS_PROMOTED", "closure flags")
    validate_closure_external_bindings(closure)
    reconciliation = mapping(closure["candidate_reconciliation"], "closure reconciliation")
    expected_total = len(rev3_rows(reader))
    if reconciliation.get("current_total") != expected_total:
        fail("AGGREGATE_COUNT_MISMATCH", "closure reconciliation current_total")


def run_prerequisite(name: str, args: list[str]) -> None:
    path = ROOT / "scripts" / name
    try:
        result = subprocess.run(
            [sys.executable, str(path), *args],
            cwd=ROOT,
            capture_output=True,
            text=True,
            env=os.environ.copy(),
        )
    except OSError as exc:
        blocked("PREREQUISITE_UNAVAILABLE", f"{name}: {exc}")
    if result.returncode != 0:
        status = "BLOCKED" if result.returncode == 2 else "FAIL"
        raise CCheckError(
            status,
            "PREREQUISITE_NOT_PASS",
            f"{name} {args}: {result.stdout[-1000:]}{result.stderr[-1000:]}",
        )


def validate_prerequisites() -> None:
    if _PERSISTENCE_IMPORT_ERROR is not None:
        blocked("CANONICAL_CODEC_UNAVAILABLE", str(_PERSISTENCE_IMPORT_ERROR))
    commands: list[tuple[str, list[str]]] = [
        ("check_m2_5_master_drift.py", []),
        ("check_m2_5_b1_authority_citations.py", []),
        ("check_m2_5_b2_classifications.py", []),
        ("check_m2_5_b1_final_authority_citations.py", []),
    ]
    for name, args in commands:
        run_prerequisite(name, args)


def validate_universe(
    snapshot: Snapshot,
    reader: ArchiveReader,
    catalog: dict[str, Any],
    model_raw: bytes,
    review_raw: bytes,
) -> tuple[list[dict[str, Any]], dict[str, dict[str, str]]]:
    universe = get_artifact(snapshot, UNIVERSE_NAME)
    exact_keys(
        universe,
        {
            "schema",
            "model_id",
            "input_bindings",
            "candidate_count",
            "candidate_reconciliation_counts",
            "source_instance_count",
            "candidates",
            "source_instances",
        },
        "candidate universe",
    )
    require(
        universe["schema"] == C_JSON_SCHEMAS[UNIVERSE_NAME],
        "SCHEMA_MISMATCH",
        "candidate universe schema",
    )
    require(
        universe["model_id"] == MODEL_ID,
        "MODEL_ID_MISMATCH",
        "candidate universe model",
    )
    exact_keys(
        universe["input_bindings"],
        {
            "declared_model",
            "review_additions",
            "rev3_candidate_source",
            "b2_artifacts",
            "b1_final_artifacts",
        },
        "candidate universe input_bindings",
    )
    require_local_input_bindings(universe, model_raw, review_raw)
    validate_b2_and_b1_bindings(
        universe["input_bindings"],
        "candidate universe input_bindings",
        "B2_CATALOG_DIGEST_MISMATCH",
        "B1_FINAL_GRAPH_DIGEST_MISMATCH",
    )
    rev3_binding = mapping(
        universe["input_bindings"].get("rev3_candidate_source"), "candidate universe REV3 source"
    )
    if rev3_binding != {
        "archive_member": EXPECTED_REV3_CENSUS_MEMBER,
        "archive_member_sha256": EXPECTED_REV3_CENSUS_SHA256,
        "source_package_sha256": EXPECTED_ARCHIVE_SHA256,
    }:
        fail("REV3_ARCHIVE_DIGEST_MISMATCH", "candidate universe REV3 source binding")
    expected, instance_lookup = expected_candidates(reader, catalog)
    known_osis = set(b2_classification_map())
    actual = list_value(universe["candidates"], "candidate universe candidates")
    review_records = {
        mapping(record, "targeted review record")["review_record_id"]: mapping(
            record, "targeted review record"
        )
        for record in list_value(
            get_artifact(snapshot, REVIEW_NAME)["review_records"], "targeted review records"
        )
    }
    for raw_candidate in actual:
        candidate_origin = mapping(raw_candidate, "candidate").get("source_origin")
        if candidate_origin == "targeted_higher_order_review":
            binding = mapping(
                mapping(raw_candidate, "targeted candidate").get("source_binding"),
                "targeted candidate source binding",
            )
            review_id = binding.get("review_record_id")
            if review_id not in review_records:
                if not review_records:
                    fail(
                        "TARGETED_REVIEW_RECORD_MISSING",
                        f"targeted candidate references {review_id!r} but no review record exists",
                    )
                fail(
                    "TARGETED_REVIEW_RECORD_UNKNOWN",
                    f"targeted candidate references unknown review record {review_id!r}",
                )
    if len(actual) != len(expected) or {
        mapping(x, "candidate").get("candidate_id") for x in actual
    } != {x["candidate_id"] for x in expected}:
        fail(
            "REV3_CANDIDATE_UNACCOUNTED",
            "candidate universe is not the complete inherited REV3 set",
        )
    expected_by_id = {x["candidate_id"]: x for x in expected}
    seen: set[str] = set()
    for index, raw_candidate in enumerate(actual):
        candidate = mapping(raw_candidate, f"candidate {index}")
        exact_keys(
            candidate,
            {
                "candidate_id",
                "candidate_identity",
                "source_origin",
                "scope",
                "relation",
                "participant_refs",
                "supporting_requirement_ids",
                "source_binding",
                "reconciliation_status",
                "reconciliation_reason",
            },
            f"candidate {index}",
        )
        cid = nonempty_string(candidate["candidate_id"], f"candidate {index}.candidate_id")
        if cid in seen:
            fail("REV3_CANDIDATE_UNACCOUNTED", f"duplicate candidate {cid}")
        seen.add(cid)
        expected_candidate = expected_by_id.get(cid)
        if expected_candidate is None:
            fail("REV3_CANDIDATE_UNACCOUNTED", f"unknown candidate {cid}")
        for field in ("source_origin", "scope", "relation"):
            actual_value = candidate[field]
            expected_value = expected_candidate[field]
            if actual_value != expected_value:
                if isinstance(actual_value, str) and actual_value.lower() == expected_value:
                    fail("NONCANONICAL_ENUM_VARIANT", f"candidate {cid}.{field}")
                if (
                    field == "relation"
                    and expected_value == "directional_binary"
                    and actual_value == "unordered_binary"
                ):
                    fail("DIRECTIONALITY_LOST", f"candidate {cid} lost directional relation")
                fail("REV3_NORMALIZATION_MISMATCH", f"candidate {cid}.{field}")
        participants = list_value(
            candidate["participant_refs"], f"candidate {cid}.participant_refs"
        )
        expected_participants = expected_candidate["participant_refs"]
        for p in participants:
            item = mapping(p, f"candidate {cid}.participant_ref")
            kind = item.get("participant_kind")
            reference = item.get("semantic_ref")
            participant_ref_cbor(item, f"candidate {cid}.participant_ref")
            if kind == "card" and not isinstance(reference, str):
                fail("OSI_UNKNOWN", f"candidate {cid} card reference")
            if (
                kind == "card"
                and isinstance(reference, str)
                and (not UUID_RE.fullmatch(reference) or reference not in known_osis)
            ):
                fail("OSI_UNKNOWN", f"candidate {cid} unknown OSI")
            if kind == "requirement_family" and reference not in {
                x["family_id"] for x in list_value(catalog.get("families"), "B2 families")
            }:
                fail("FAMILY_UNKNOWN", f"candidate {cid} unknown family")
        if participants != expected_participants:
            if (
                len(expected_participants) == 2
                and participants == list(reversed(expected_participants))
                and expected_candidate["relation"] == "directional_binary"
            ):
                fail("DIRECTION_REVERSED", f"candidate {cid} direction reversed")
            fail("REV3_NORMALIZATION_MISMATCH", f"candidate {cid}.participant_refs")
        supports = list_value(
            candidate["supporting_requirement_ids"], f"candidate {cid}.supporting_requirement_ids"
        )
        if supports != expected_candidate["supporting_requirement_ids"]:
            fail("REV3_NORMALIZATION_MISMATCH", f"candidate {cid}.supporting_requirement_ids")
        binding = mapping(candidate["source_binding"], f"candidate {cid}.source_binding")
        if binding.get("kind") == "b2_derived":
            fail("B2_DERIVED_FORBIDDEN_V1", f"candidate {cid} uses b2_derived")
        if (
            binding.get("kind") == "rev3"
            and isinstance(binding.get("archive_member_sha256"), str)
            and HEX64_RE.fullmatch(binding["archive_member_sha256"]) is None
        ):
            fail("SHA256_SCALAR_ENCODING_INVALID", f"candidate {cid}.archive_member_sha256")
        source_binding_cbor(binding, f"candidate {cid}.source_binding")
        if binding != expected_candidate["source_binding"]:
            fail("SOURCE_BINDING_INVALID", f"candidate {cid}.source_binding")
        validate_digest_ref(candidate["candidate_identity"], f"candidate {cid}.candidate_identity")
        expected_identity = expected_candidate["candidate_identity"]
        if candidate["candidate_identity"] != expected_identity:
            fail("CANDIDATE_IDENTITY_MISMATCH", f"candidate {cid} identity digest")
        require(
            candidate["reconciliation_status"] == "unchanged",
            "REV3_RECONCILIATION_INVALID",
            f"candidate {cid} status",
        )
        nonempty_string(
            candidate["reconciliation_reason"], f"candidate {cid} reconciliation reason"
        )
    require(
        universe["candidate_count"] == len(actual), "AGGREGATE_COUNT_MISMATCH", "candidate_count"
    )
    counts = mapping(universe["candidate_reconciliation_counts"], "candidate reconciliation counts")
    expected_counts = {
        "unchanged": len(expected),
        "stale_rev3_candidate": 0,
        "removed_not_interaction": 0,
        "merged_semantic_duplicate": 0,
        "new_targeted_higher_order_candidate": 0,
        "new_b2_derived": 0,
    }
    require(
        counts == expected_counts, "AGGREGATE_COUNT_MISMATCH", "candidate reconciliation counts"
    )
    instances = list_value(universe["source_instances"], "source instances")
    by_instance: dict[str, dict[str, Any]] = {}
    source_instance_tuples: set[bytes] = set()
    for index, raw_instance in enumerate(instances):
        instance = mapping(raw_instance, f"source instance {index}")
        exact_keys(
            instance,
            {
                "source_instance_id",
                "candidate_id",
                "source_binding",
                "participant_bindings",
                "source_context",
            },
            f"source instance {index}",
        )
        iid = nonempty_string(instance["source_instance_id"], f"source instance {index}.id")
        cid = string(instance["candidate_id"], f"source instance {index}.candidate_id")
        if cid not in expected_by_id:
            fail("ORPHAN_SOURCE_INSTANCE", f"source instance {iid} has no candidate")
        instance_participants = list_value(
            instance["participant_bindings"], f"source instance {iid}.participant_bindings"
        )
        instance_context = context_cbor(
            instance["source_context"], f"source instance {iid}.source_context"
        )
        tuple_key = canonical(
            [
                cid,
                [
                    participant_binding_cbor(
                        value, f"source instance {iid}.participant_bindings[{p_index}]"
                    )
                    for p_index, value in enumerate(instance_participants)
                ],
                instance_context,
            ]
        )
        if tuple_key in source_instance_tuples:
            fail(
                "DUPLICATE_SOURCE_INSTANCE_TUPLE",
                f"source instance {iid} duplicates a canonical tuple",
            )
        source_instance_tuples.add(tuple_key)
        if iid in by_instance:
            fail("ORPHAN_SOURCE_INSTANCE", f"duplicate source instance {iid}")
        if iid != instance_lookup[cid]["source_instance_id"]:
            fail("ORPHAN_SOURCE_INSTANCE", f"source instance ID is not deterministic for {cid}")
        if instance["source_binding"] != expected_by_id[cid]["source_binding"]:
            fail("SOURCE_BINDING_INVALID", f"source instance {iid} binding")
        source_binding_cbor(instance["source_binding"], f"source instance {iid}.source_binding")
        bindings = instance_participants
        expected_refs = expected_by_id[cid]["participant_refs"]
        require(
            len(bindings) == len(expected_refs),
            "PARTICIPANT_ROLE_MISSING",
            f"source instance {iid} participant bindings",
        )
        for p_index, item in enumerate(bindings):
            participant_binding_cbor(item, f"source instance {iid}.participant_bindings[{p_index}]")
            require(
                item["participant_ref"] == expected_refs[p_index],
                "PARTICIPANT_ROLE_MISSING",
                f"source instance {iid} participant binding",
            )
        by_instance[iid] = instance
    if len(by_instance) != len(actual):
        fail(
            "ORPHAN_SOURCE_INSTANCE",
            "source instance ledger does not cover every candidate exactly once",
        )
    require(
        universe["source_instance_count"] == len(instances),
        "AGGREGATE_COUNT_MISMATCH",
        "source_instance_count",
    )
    return expected, instance_lookup


def validate_classes(
    snapshot: Snapshot,
    classes: dict[str, Any],
    catalog: dict[str, Any],
    b1: dict[str, Any],
    b2_by_osi: dict[str, dict[str, Any]],
) -> dict[str, dict[str, Any]]:
    exact_keys(
        classes,
        {"schema", "model_id", "input_bindings", "class_count", "classes"},
        "semantic classes",
    )
    require(
        classes["schema"] == C_JSON_SCHEMAS[CLASSES_NAME],
        "SCHEMA_MISMATCH",
        "semantic classes schema",
    )
    require(
        classes["model_id"] == MODEL_ID,
        "MODEL_ID_MISMATCH",
        "semantic classes model",
    )
    bindings = mapping(classes["input_bindings"], "semantic classes input_bindings")
    exact_keys(
        bindings,
        {"candidate_universe", "b2_artifacts", "b1_final_artifacts"},
        "semantic classes input_bindings",
    )
    candidate_binding = mapping(
        bindings["candidate_universe"], "semantic classes candidate binding"
    )
    exact_keys(candidate_binding, {"path", "raw_sha256"}, "semantic classes candidate binding")
    require(
        candidate_binding["path"]
        == "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
        "PREREQUISITE_IDENTITY_STALE",
        "semantic class candidate path",
    )
    require(
        candidate_binding["raw_sha256"] == sha256_bytes(snapshot.raw[UNIVERSE_NAME]),
        "PREREQUISITE_IDENTITY_STALE",
        "semantic class candidate binding",
    )
    validate_b2_and_b1_bindings(
        bindings,
        "semantic classes input_bindings",
        "B2_CATALOG_DIGEST_MISMATCH",
        "B1_FINAL_GRAPH_DIGEST_MISMATCH",
    )
    catalog_by_id, active, active_unassigned = b2_family_maps(catalog)
    citations = {
        a["authority_id"]: {c["citation_id"] for c in a.get("citations", [])}
        for a in list_value(b1.get("authorities"), "B1 authorities")
    }
    records = list_value(classes["classes"], "semantic class records")
    require(classes["class_count"] == len(records), "AGGREGATE_COUNT_MISMATCH", "class_count")
    result: dict[str, dict[str, Any]] = {}
    for index, raw_record in enumerate(records):
        record = mapping(raw_record, f"class {index}")
        exact_keys(
            record,
            {
                "interaction_class_id",
                "class_identity",
                "arity",
                "directionality",
                "participant_roles",
                "host_relationship",
                "context_dimensions",
                "temporal_semantics",
                "b2_family_refs",
                "b2_boundary_refs",
                "b1_final_citation_refs",
                "semantic_rationale",
                "source_evidence_refs",
            },
            f"class {index}",
        )
        cid = nonempty_string(record["interaction_class_id"], f"class {index}.id")
        if cid in result:
            fail("DUPLICATE_CLASS_ID", f"duplicate class ID {cid}")
        validate_digest_ref(record["class_identity"], f"class {cid}.identity")
        roles = list_value(record["participant_roles"], f"class {cid}.roles")
        for role_index, raw_role in enumerate(roles):
            role = mapping(raw_role, f"class {cid}.role {role_index}")
            exact_keys(
                role,
                {"position", "role", "participant_kind", "semantic_ref"},
                f"class {cid}.role {role_index}",
            )
            if not isinstance(role["position"], int) or isinstance(role["position"], bool):
                fail("PARTICIPANT_ROLE_MISSING", f"class {cid} role position is not an integer")
            enum_value(role["role"], PARTICIPANT_ROLES, f"class {cid}.role {role_index}.role")
            participant_ref_cbor(
                {
                    "participant_kind": role["participant_kind"],
                    "semantic_ref": role["semantic_ref"],
                },
                f"class {cid}.role {role_index}.participant_ref",
            )
        positions = [mapping(x, "class role").get("position") for x in roles]
        if positions != list(range(len(roles))):
            fail("PARTICIPANT_ROLE_MISSING", f"class {cid} roles are not complete and ordered")
        arity = enum_value(record["arity"], EXTRA_VOCABULARY["arity"], f"class {cid}.arity")
        if (arity == "unary" and len(roles) != 1) or (arity == "binary" and len(roles) != 2):
            fail("PARTICIPANT_ROLE_MISSING", f"class {cid} arity does not match role count")
        if arity == "higher_order" and len(roles) <= 2:
            fail(
                "HIGHER_ORDER_PARTICIPANT_MISSING",
                f"class {cid} has no finite higher-order participant set",
            )
        if len(roles) <= 1 and arity == "higher_order":
            fail(
                "HIGHER_ORDER_PARTICIPANT_MISSING", f"class {cid} higher-order participant missing"
            )
        enum(
            record["directionality"],
            EXTRA_VOCABULARY["directionality"],
            f"class {cid}.directionality",
        )
        host = enum_value(
            record["host_relationship"],
            EXTRA_VOCABULARY["host_relationship"],
            f"class {cid}.host_relationship",
        )
        if host != "not_applicable" and arity == "unary":
            fail(
                "HOST_RELATIONSHIP_MISMATCH",
                f"class {cid} unary host relationship must be not_applicable",
            )
        context_cbor(record["context_dimensions"], f"class {cid}.context_dimensions")
        temporal_cbor(record["temporal_semantics"], f"class {cid}.temporal_semantics")
        family_refs = list_value(record["b2_family_refs"], f"class {cid}.b2_family_refs")
        boundary_refs = list_value(record["b2_boundary_refs"], f"class {cid}.b2_boundary_refs")
        boundary_by_id: dict[str, str] = {}
        for item in family_refs:
            ref = mapping(item, "class B2 family ref")
            exact_keys(ref, {"family_id", "lifecycle", "assignment_role"}, "class B2 family ref")
            family_id = string(ref["family_id"], "class B2 family ID")
            if family_id not in catalog_by_id:
                fail("FAMILY_UNKNOWN", f"class {cid} references {family_id}")
            if ref["lifecycle"] == "active_unassigned" or family_id in active_unassigned:
                fail("ACTIVE_UNASSIGNED_CARD_DERIVED", f"class {cid} uses ACTIVE_UNASSIGNED family")
            require(
                family_id in active and ref["lifecycle"] == "active",
                "ASSIGNMENT_BINDING_INVALID",
                f"class {cid} family lifecycle",
            )
            enum(
                ref["assignment_role"], EXTRA_VOCABULARY["assignment_role"], "class assignment role"
            )
        for item in boundary_refs:
            ref = mapping(item, "class B2 boundary ref")
            exact_keys(ref, {"family_id", "precise_semantic_definition"}, "class B2 boundary ref")
            family_id = string(ref["family_id"], "class B2 boundary ID")
            if family_id not in catalog_by_id:
                fail("FAMILY_UNKNOWN", f"class {cid} boundary family {family_id}")
            expected_boundary = catalog_by_id[family_id].get("precise_semantic_definition")
            if ref["precise_semantic_definition"] != expected_boundary:
                fail("B2_BOUNDARY_BINDING_MISMATCH", f"class {cid} boundary {family_id}")
            boundary_by_id[family_id] = ref["precise_semantic_definition"]
        if set(boundary_by_id) != {mapping(x, "family ref")["family_id"] for x in family_refs}:
            fail("B2_BOUNDARY_BINDING_MISMATCH", f"class {cid} boundary set")
        citation_refs = b1_citation_sort(
            list_value(record["b1_final_citation_refs"], f"class {cid}.citations"),
            f"class {cid}.citations",
        )
        if not citation_refs and arity == "unary":
            fail("B1_CITATION_UNRESOLVED", f"class {cid} has no B1.Final citation")
        for citation in citation_refs:
            pair = mapping(citation, "B1 citation")
            authority_id = pair["authority_id"]
            if authority_id not in citations or pair["citation_id"] not in citations[authority_id]:
                fail("B1_CITATION_UNRESOLVED", f"class {cid} cites unknown B1.Final node")
        class_evidence = list_value(record["source_evidence_refs"], f"class {cid}.evidence")
        for evidence in class_evidence:
            if mapping(evidence, "class evidence")["authority_kind"] not in {
                "rev3",
                "b2",
                "b1_final",
                "c_review",
            }:
                fail("UNAPPROVED_AUTHORITY", f"class {cid} evidence authority")
        evidence_sort(class_evidence, f"class {cid}.evidence")
        nonempty_string(record["semantic_rationale"], f"class {cid}.rationale")
        expected_identity = make_class_identity(record)
        require(
            record["interaction_class_id"] == f"ic.v1/{expected_identity['digest_hex']}",
            "CANDIDATE_IDENTITY_MISMATCH",
            f"class {cid} identity",
        )
        require(
            record["class_identity"] == expected_identity,
            "CANDIDATE_IDENTITY_MISMATCH",
            f"class {cid} digest",
        )
        result[cid] = record
    universe_candidates = list_value(
        get_artifact(snapshot, UNIVERSE_NAME)["candidates"], "candidate universe candidates"
    )
    expected_class_osis = {
        mapping(candidate, "candidate")["participant_refs"][0]["semantic_ref"]
        for candidate in universe_candidates
        if mapping(candidate, "candidate")["relation"] == "declared_card_trigger"
    }
    actual_class_osis = {
        mapping(record["participant_roles"][0], "class participant role")["semantic_ref"]
        for record in result.values()
    }
    require(
        actual_class_osis == expected_class_osis,
        "CANDIDATE_CLASS_UNRESOLVED",
        "semantic classes do not cover exactly the card-trigger candidates",
    )
    return result


def validate_classification_bundle(
    snapshot: Snapshot,
    root: dict[str, Any],
    expected_candidates_list: list[dict[str, Any]],
) -> dict[str, Any]:
    """Validate the V3 root/shard authority and return its ordered records."""
    if "candidate_classifications" in root or "classification_root_raw_sha256" in root:
        fail(
            "CLASSIFICATION_LAYOUT_INVALID",
            "classification root contains record data or a self-referential digest",
        )
    exact_keys(
        root,
        {
            "schema",
            "model_id",
            "candidate_universe_raw_sha256",
            "semantic_classes_raw_sha256",
            "partition_scheme",
            "classification_count",
            "shard_count",
            "shards",
        },
        "classification root",
    )
    require(
        root["schema"] == C_JSON_SCHEMAS[CLASSIFICATIONS_NAME],
        "SCHEMA_MISMATCH",
        "classification root schema",
    )
    require(root["model_id"] == MODEL_ID, "MODEL_ID_MISMATCH", "classification root model")
    require(
        root["candidate_universe_raw_sha256"] == sha256_bytes(snapshot.raw[UNIVERSE_NAME]),
        "PREREQUISITE_IDENTITY_STALE",
        "classification root candidate-universe binding",
    )
    require(
        root["semantic_classes_raw_sha256"] == sha256_bytes(snapshot.raw[CLASSES_NAME]),
        "PREREQUISITE_IDENTITY_STALE",
        "classification root semantic-class binding",
    )
    require(
        root["partition_scheme"] == PARTITION_SCHEME,
        "CLASSIFICATION_SHARD_RANGE_MISMATCH",
        "classification root partition scheme",
    )
    require(
        root["classification_count"] == EXPECTED_REV3_CANDIDATE_COUNT,
        "CLASSIFICATION_ROOT_COUNT_MISMATCH",
        "classification root count",
    )
    require(
        root["shard_count"] == SHARD_COUNT,
        "CLASSIFICATION_ROOT_COUNT_MISMATCH",
        "classification root shard count",
    )
    root_entries = list_value(root["shards"], "classification root shards")
    if len(root_entries) < SHARD_COUNT:
        fail("CLASSIFICATION_SHARD_MISSING", "classification root shard inventory is incomplete")
    if len(root_entries) > SHARD_COUNT:
        fail("UNLISTED_C_ARTIFACT", "classification root contains an extra shard entry")
    ordered_candidates = sorted(
        expected_candidates_list, key=lambda candidate: canonical(candidate["candidate_id"])
    )
    expected_ids = [candidate["candidate_id"] for candidate in ordered_candidates]
    records: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, relative in enumerate(SHARD_NAMES):
        root_entry = mapping(root_entries[index], f"classification root shard {index}")
        exact_keys(
            root_entry,
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
            f"classification root shard {index}",
        )
        start = 1000 * index
        end = min(1000 * (index + 1), EXPECTED_REV3_CANDIDATE_COUNT)
        require(
            root_entry
            == {
                "shard_index": index,
                "path": relative,
                "ordinal_start": start,
                "ordinal_end_exclusive": end,
                "record_count": end - start,
                "first_candidate_id": expected_ids[start],
                "last_candidate_id": expected_ids[end - 1],
                "raw_sha256": root_entry["raw_sha256"],
            },
            "CLASSIFICATION_SHARD_RANGE_MISMATCH",
            f"classification root shard {index} metadata",
        )
        if relative not in snapshot.values or relative not in snapshot.raw:
            fail("CLASSIFICATION_SHARD_MISSING", f"missing classification shard {relative}")
        shard = get_artifact(snapshot, relative)
        if "classification_root_raw_sha256" in shard:
            fail(
                "CLASSIFICATION_LAYOUT_INVALID",
                f"classification shard {index} contains a root digest",
            )
        exact_keys(
            shard,
            {
                "schema",
                "model_id",
                "candidate_universe_raw_sha256",
                "semantic_classes_raw_sha256",
                "partition_scheme",
                "shard_index",
                "shard_count",
                "ordinal_start",
                "ordinal_end_exclusive",
                "record_count",
                "first_candidate_id",
                "last_candidate_id",
                "candidate_classifications",
            },
            f"classification shard {index}",
        )
        validate_shard_size(len(snapshot.raw[relative]), index)
        require(
            sha256_bytes(snapshot.raw[relative]) == root_entry["raw_sha256"],
            "CLASSIFICATION_SHARD_DIGEST_MISMATCH",
            f"classification shard {index} raw digest",
        )
        for key in (
            "schema",
            "model_id",
            "candidate_universe_raw_sha256",
            "semantic_classes_raw_sha256",
            "partition_scheme",
            "shard_index",
            "shard_count",
            "ordinal_start",
            "ordinal_end_exclusive",
            "record_count",
            "first_candidate_id",
            "last_candidate_id",
        ):
            expected_value = (
                C_JSON_SCHEMAS[SHARD_SCHEMA_NAME]
                if key == "schema"
                else root_entry[key]
                if key in root_entry
                else root[key]
                if key
                in {
                    "model_id",
                    "candidate_universe_raw_sha256",
                    "semantic_classes_raw_sha256",
                    "shard_count",
                }
                else PARTITION_SCHEME
            )
            require(
                shard[key] == expected_value,
                "CLASSIFICATION_SHARD_RANGE_MISMATCH",
                f"classification shard {index}.{key}",
            )
        shard_records = list_value(
            shard["candidate_classifications"], f"classification shard {index}.records"
        )
        require(
            len(shard_records) == end - start == shard["record_count"],
            "CLASSIFICATION_SHARD_COVERAGE_MISMATCH",
            f"classification shard {index}.record_count",
        )
        actual_ids: list[str] = []
        for record_index, raw_record in enumerate(shard_records):
            record = mapping(raw_record, f"classification shard {index} record {record_index}")
            candidate_id = nonempty_string(
                record.get("candidate_id"),
                f"classification shard {index} record {record_index}.candidate_id",
            )
            if candidate_id in seen:
                fail(
                    "CLASSIFICATION_SHARD_COVERAGE_MISMATCH",
                    f"duplicate candidate {candidate_id}",
                )
            seen.add(candidate_id)
            actual_ids.append(candidate_id)
        expected_shard_ids = expected_ids[start:end]
        if actual_ids != expected_shard_ids:
            if set(actual_ids) != set(expected_shard_ids):
                fail(
                    "CLASSIFICATION_SHARD_COVERAGE_MISMATCH",
                    f"classification shard {index} candidate coverage",
                )
            fail(
                "CLASSIFICATION_LAYOUT_INVALID",
                f"classification shard {index} candidate order",
            )
        require(
            shard["first_candidate_id"] == actual_ids[0]
            and shard["last_candidate_id"] == actual_ids[-1],
            "CLASSIFICATION_SHARD_RANGE_MISMATCH",
            f"classification shard {index} candidate boundaries",
        )
        records.extend(cast(list[dict[str, Any]], shard_records))
    require(
        len(records) == root["classification_count"] == len(expected_ids)
        and seen == set(expected_ids),
        "CLASSIFICATION_SHARD_COVERAGE_MISMATCH",
        "classification shards do not cover the candidate universe exactly",
    )
    return {
        "schema": root["schema"],
        "model_id": root["model_id"],
        "candidate_universe_raw_sha256": root["candidate_universe_raw_sha256"],
        "semantic_classes_raw_sha256": root["semantic_classes_raw_sha256"],
        "classification_count": root["classification_count"],
        "candidate_classifications": records,
    }


def validate_classifications(
    snapshot: Snapshot,
    classifications: dict[str, Any],
    expected_candidates_list: list[dict[str, Any]],
    instance_lookup: dict[str, dict[str, str]],
    classes_by_id: dict[str, dict[str, Any]],
    catalog: dict[str, Any],
    b1: dict[str, Any],
) -> Counter[str]:
    exact_keys(
        classifications,
        {
            "schema",
            "model_id",
            "candidate_universe_raw_sha256",
            "semantic_classes_raw_sha256",
            "classification_count",
            "candidate_classifications",
        },
        "classifications",
    )
    require(
        classifications["schema"] == C_JSON_SCHEMAS[CLASSIFICATIONS_NAME],
        "SCHEMA_MISMATCH",
        "classifications schema",
    )
    require(
        classifications["model_id"] == MODEL_ID,
        "MODEL_ID_MISMATCH",
        "classifications model",
    )
    require(
        classifications["candidate_universe_raw_sha256"]
        == sha256_bytes(snapshot.raw[UNIVERSE_NAME]),
        "PREREQUISITE_IDENTITY_STALE",
        "classification universe binding",
    )
    require(
        classifications["semantic_classes_raw_sha256"] == sha256_bytes(snapshot.raw[CLASSES_NAME]),
        "PREREQUISITE_IDENTITY_STALE",
        "classification class binding",
    )
    records = list_value(classifications["candidate_classifications"], "candidate classifications")
    require(
        classifications["classification_count"] == len(records),
        "AGGREGATE_COUNT_MISMATCH",
        "classification_count",
    )
    expected = {x["candidate_id"]: x for x in expected_candidates_list}
    seen: set[str] = set()
    metrics = Counter[str]()
    allowed_citations = {
        a["authority_id"]: {c["citation_id"] for c in a.get("citations", [])}
        for a in list_value(b1.get("authorities"), "B1 authorities")
    }
    catalog_by_id, _, _ = b2_family_maps(catalog)
    known_instance_ids = {value["source_instance_id"] for value in instance_lookup.values()}
    instance_owner = {
        value["source_instance_id"]: candidate_id for candidate_id, value in instance_lookup.items()
    }
    for index, raw_record in enumerate(records):
        record = mapping(raw_record, f"classification {index}")
        exact_keys(
            record,
            {
                "candidate_id",
                "review_state",
                "review_domain_assessments",
                "terminal_disposition",
                "interaction_class_id",
                "unresolved_reason",
                "source_instance_context_mappings",
                "reconciliation",
                "review_rationale",
                "evidence_refs",
            },
            f"classification {index}",
        )
        cid = nonempty_string(record["candidate_id"], f"classification {index}.candidate_id")
        if cid in seen or cid not in expected:
            fail("REV3_CANDIDATE_UNACCOUNTED", f"classification candidate set issue for {cid}")
        seen.add(cid)
        validate_v2_candidate_state(record, f"classification {cid}")
        state = string(record["review_state"], f"classification {cid}.review_state")
        disposition = record["terminal_disposition"]
        if state == "resolved":
            metrics[f"resolved_{disposition}"] += 1
        else:
            metrics["unresolved"] += 1
        class_id = record["interaction_class_id"]
        if disposition == "required_interaction" and class_id not in classes_by_id:
            fail("CANDIDATE_CLASS_UNRESOLVED", f"required candidate {cid} has no class")

        assessments = list_value(
            record["review_domain_assessments"], f"classification {cid}.review_domain_assessments"
        )
        require(
            len(assessments) == len(REVIEW_DOMAINS),
            "REVIEW_DOMAIN_ASSESSMENT_INCOMPLETE",
            f"classification {cid} review-domain assessment count",
        )
        has_unresolved_domain = False
        for domain_index, raw_assessment in enumerate(assessments):
            assessment = mapping(
                raw_assessment, f"classification {cid}.review domain {domain_index}"
            )
            exact_keys(
                assessment,
                {"review_domain", "applicability", "evidence_refs"},
                f"classification {cid}.review domain {domain_index}",
            )
            require(
                assessment["review_domain"] == REVIEW_DOMAINS[domain_index],
                "REVIEW_DOMAIN_ASSESSMENT_INCOMPLETE",
                f"classification {cid} review-domain order",
            )
            enum_value(
                assessment["review_domain"], REVIEW_DOMAINS, f"classification {cid}.review_domain"
            )
            applicability = enum_value(
                assessment["applicability"],
                REVIEW_DOMAIN_APPLICABILITY,
                f"classification {cid}.review_domain.applicability",
            )
            domain_evidence = list_value(
                assessment["evidence_refs"], f"classification {cid}.review_domain.evidence"
            )
            if not domain_evidence:
                fail(
                    "MISSING_REQUIRED_REVIEW_EVIDENCE",
                    f"classification {cid} has no evidence for {assessment['review_domain']}",
                )
            for evidence in domain_evidence:
                if mapping(evidence, "review-domain evidence").get("authority_kind") not in {
                    "rev3",
                    "b2",
                    "b1_final",
                    "c_review",
                }:
                    fail("UNAPPROVED_AUTHORITY", f"classification {cid} domain evidence authority")
            evidence_sort(domain_evidence, f"classification {cid}.review_domain.evidence")
            has_unresolved_domain |= applicability == "unresolved"
        if state == "resolved" and has_unresolved_domain:
            fail(
                "NONTERMINAL_DISPOSITION_ON_PASS",
                f"resolved classification {cid} has an unresolved review domain",
            )
        if state == "unresolved":
            unresolved_reason = string(
                record["unresolved_reason"], f"classification {cid}.unresolved_reason"
            )
            if expected[cid]["relation"] in {"unordered_binary", "directional_binary"}:
                require(
                    unresolved_reason == "insufficient_pair_relation_authority",
                    "UNRESOLVED_REASON_INVALID",
                    f"classification {cid} unresolved reason precedence",
                )
            elif has_unresolved_domain:
                require(
                    unresolved_reason == "missing_required_review_evidence",
                    "UNRESOLVED_REASON_INVALID",
                    f"classification {cid} unresolved reason precedence",
                )
        mappings = list_value(
            record["source_instance_context_mappings"], f"classification {cid}.mappings"
        )
        if not mappings:
            fail(
                "DUPLICATE_SOURCE_INSTANCE_MAPPING",
                f"candidate {cid} has no source-instance mapping",
            )
        mapping_keys: set[bytes] = set()
        for m_index, raw_mapping in enumerate(mappings):
            item = mapping(raw_mapping, f"classification {cid}.mapping {m_index}")
            exact_keys(
                item,
                {
                    "source_instance_id",
                    "participant_bindings",
                    "context_binding",
                    "b2_assignment_refs",
                    "b1_final_citation_refs",
                },
                f"classification {cid}.mapping {m_index}",
            )
            instance_id = nonempty_string(
                item["source_instance_id"], f"classification {cid}.source_instance_id"
            )
            if instance_id not in known_instance_ids:
                fail(
                    "ORPHAN_SOURCE_INSTANCE",
                    f"classification {cid} references unknown source instance",
                )
            if instance_owner[instance_id] != cid:
                fail(
                    "ORPHAN_SOURCE_INSTANCE",
                    f"classification {cid} references an instance owned by "
                    f"{instance_owner[instance_id]}",
                )
            participant_payload = [
                participant_binding_cbor(
                    value, f"classification {cid}.mapping {m_index}.participant_bindings[{p_index}]"
                )
                for p_index, value in enumerate(
                    list_value(
                        item["participant_bindings"], f"classification {cid}.participant_bindings"
                    )
                )
            ]
            context_payload = context_cbor(
                item["context_binding"], f"classification {cid}.mapping {m_index}.context_binding"
            )
            tuple_key = canonical([instance_id, participant_payload, context_payload])
            if tuple_key in mapping_keys:
                fail(
                    "DUPLICATE_SOURCE_INSTANCE_MAPPING",
                    f"classification {cid} maps the same instance twice",
                )
            mapping_keys.add(tuple_key)
            participant_bindings = list_value(
                item["participant_bindings"], f"classification {cid}.participant_bindings"
            )
            expected_role = (
                "trigger_source"
                if expected[cid]["relation"] == "declared_card_trigger"
                else "ordered_participant"
            )
            expected_participant_bindings = [
                {"role": expected_role, "participant_ref": copy.deepcopy(ref)}
                for ref in expected[cid]["participant_refs"]
            ]
            if participant_bindings != expected_participant_bindings:
                fail(
                    "PARTICIPANT_ROLE_MISSING",
                    f"classification {cid} participant binding does not match its source instance",
                )
            if item["context_binding"] != context_not_applicable():
                fail(
                    "CONTEXT_DIMENSION_MISSING",
                    f"classification {cid} context binding differs from the source instance",
                )
            assignments = list_value(
                item["b2_assignment_refs"], f"classification {cid}.b2_assignment_refs"
            )
            for a_index, assignment in enumerate(assignments):
                ref = mapping(assignment, f"classification {cid}.assignment {a_index}")
                exact_keys(
                    ref,
                    {"family_id", "assignment_ordinal", "precise_semantic_definition"},
                    f"classification {cid}.assignment {a_index}",
                )
                if ref["family_id"] not in catalog_by_id:
                    fail("FAMILY_UNKNOWN", f"classification {cid} assignment family")
                if not isinstance(ref["assignment_ordinal"], int) or ref["assignment_ordinal"] < 0:
                    fail("ASSIGNMENT_BINDING_INVALID", f"classification {cid} assignment ordinal")
                expected_boundary = catalog_by_id[ref["family_id"]].get(
                    "precise_semantic_definition"
                )
                if ref["precise_semantic_definition"] != expected_boundary:
                    fail("ASSIGNMENT_BINDING_INVALID", f"classification {cid} assignment boundary")
            for citation in b1_citation_sort(
                list_value(item["b1_final_citation_refs"], f"classification {cid}.citations"),
                f"classification {cid}.citations",
            ):
                pair = mapping(citation, "classification citation")
                if (
                    pair["authority_id"] not in allowed_citations
                    or pair["citation_id"] not in allowed_citations[pair["authority_id"]]
                ):
                    fail("B1_CITATION_UNRESOLVED", f"classification {cid} citation")
            if disposition == "required_interaction":
                expected_citations = classes_by_id[class_id]["b1_final_citation_refs"]
                if item["b1_final_citation_refs"] != expected_citations:
                    fail(
                        "B1_CITATION_UNRESOLVED",
                        f"classification {cid} citation binding differs from its class",
                    )
            elif item["b1_final_citation_refs"]:
                fail(
                    "B1_CITATION_UNRESOLVED",
                    f"non-required classification {cid} has B1.Final citation bindings",
                )
        classification_evidence = list_value(
            record["evidence_refs"], f"classification {cid}.evidence"
        )
        if not classification_evidence:
            fail("MISSING_REQUIRED_REVIEW_EVIDENCE", f"classification {cid} has no evidence")
        for evidence in classification_evidence:
            authority = mapping(evidence, "classification evidence")["authority_kind"]
            if authority not in {"rev3", "b2", "b1_final", "c_review"}:
                fail("UNAPPROVED_AUTHORITY", f"classification {cid} evidence authority")
        evidence_sort(classification_evidence, f"classification {cid}.evidence")
        rationale = nonempty_string(record["review_rationale"], f"classification {cid}.rationale")
        if state == "unresolved":
            if cid not in rationale:
                fail(
                    "REVIEW_RATIONALE_NOT_CANDIDATE_SPECIFIC",
                    f"classification {cid} rationale omits the candidate ID",
                )
            if expected[cid]["relation"] in {"unordered_binary", "directional_binary"}:
                for participant in expected[cid]["participant_refs"]:
                    if participant["semantic_ref"] not in rationale:
                        fail(
                            "REVIEW_RATIONALE_NOT_CANDIDATE_SPECIFIC",
                            f"classification {cid} rationale omits {participant['semantic_ref']}",
                        )
        if disposition == "not_an_interaction_with_proof":
            required_boundary_refs = {
                (
                    participant["semantic_ref"],
                    "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
                )
                for participant in expected[cid]["participant_refs"]
                if participant["participant_kind"] == "requirement_family"
            }
            actual_boundary_refs = {
                (item["locator"][1], item["path"])
                for item in classification_evidence
                if item.get("authority_kind") == "b2"
                and isinstance(item.get("locator"), list)
                and len(item["locator"]) == 2
                and item["locator"][0] == "family_id"
            }
            if not required_boundary_refs.issubset(actual_boundary_refs):
                fail(
                    "INSUFFICIENT_NONINTERACTION_PROOF",
                    f"classification {cid} lacks positive B2 boundary evidence",
                )
        reconciliation = mapping(record["reconciliation"], f"classification {cid}.reconciliation")
        exact_keys(
            reconciliation,
            {"status", "original_rev3_candidate_id", "linkage"},
            f"classification {cid}.reconciliation",
        )
        require(
            reconciliation["status"] == expected[cid]["reconciliation_status"]
            and reconciliation["original_rev3_candidate_id"] == cid,
            "REV3_RECONCILIATION_INVALID",
            f"classification {cid} lineage",
        )
    require(seen == set(expected), "REV3_CANDIDATE_UNACCOUNTED", "classification set is incomplete")
    return metrics


def validate_closure(
    snapshot: Snapshot,
    closure: dict[str, Any],
    model_raw: bytes,
    review_raw: bytes,
    universe_raw: bytes,
    classes_raw: bytes,
    classifications_raw: bytes,
    review_metrics: Counter[str],
    candidate_count: int,
    class_count: int,
) -> None:
    exact_keys(
        closure,
        {
            "schema",
            "model_id",
            "bound_semantic_inputs",
            "external_prerequisite_identities",
            "candidate_reconciliation",
            "semantic_class_metrics",
            "review_state_metrics",
            "source_instance_metrics",
            "gate_status",
            "flags",
        },
        "interaction closure",
    )
    require(closure["schema"] == C_JSON_SCHEMAS[CLOSURE_NAME], "SCHEMA_MISMATCH", "closure schema")
    gate_status = mapping(closure["gate_status"], "closure gate status")
    for gate in ("CLASSIFICATION_REFERENCE_CLOSURE", "OFFICIAL_RULE_CITATION_CLOSURE"):
        if gate_status.get(gate) != "PASS":
            fail("PREREQUISITE_NOT_PASS", f"required prerequisite gate {gate} is not PASS")
    validate_closure_external_bindings(closure)
    inputs = list_value(closure["bound_semantic_inputs"], "closure semantic inputs")
    expected_paths = [
        (
            "sources/m2_5/closures/C/declared_interaction_model.v2.json",
            C_JSON_SCHEMAS[MODEL_NAME],
            sha256_bytes(model_raw),
            1,
        ),
        (
            "sources/m2_5/closures/C/interaction_review_additions.v2.json",
            C_JSON_SCHEMAS[REVIEW_NAME],
            sha256_bytes(review_raw),
            0,
        ),
        (
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            C_JSON_SCHEMAS[UNIVERSE_NAME],
            sha256_bytes(universe_raw),
            candidate_count,
        ),
        (
            "sources/m2_5/closures/C/interaction_semantic_classes.v2.json",
            C_JSON_SCHEMAS[CLASSES_NAME],
            sha256_bytes(classes_raw),
            class_count,
        ),
        (
            "sources/m2_5/closures/C/interaction_classifications.v3.json",
            C_JSON_SCHEMAS[CLASSIFICATIONS_NAME],
            sha256_bytes(classifications_raw),
            candidate_count,
        ),
    ]
    require(
        len(inputs) == len(expected_paths),
        "CLOSURE_INPUT_SET_INVALID",
        "closure must bind five inputs",
    )
    for index, (expected_path, expected_schema, expected_sha, expected_count) in enumerate(
        expected_paths
    ):
        item = mapping(inputs[index], f"closure input {index}")
        exact_keys(item, {"path", "schema", "raw_sha256", "record_count"}, f"closure input {index}")
        if (
            item["path"] != expected_path
            or item["schema"] != expected_schema
            or item["raw_sha256"] != expected_sha
            or item["record_count"] != expected_count
        ):
            fail("CLOSURE_INPUT_BINDING_MISMATCH", f"closure input {index}")
    reconciliation = mapping(closure["candidate_reconciliation"], "closure reconciliation")
    expected_reconciliation = {
        "rev3_total": candidate_count,
        "rev3_unchanged": candidate_count,
        "rev3_stale": 0,
        "rev3_removed_not_interaction": 0,
        "rev3_merged_semantic_duplicate": 0,
        "new_b2_derived": 0,
        "new_targeted_higher_order": 0,
        "current_total": candidate_count,
    }
    require(
        reconciliation == expected_reconciliation,
        "AGGREGATE_COUNT_MISMATCH",
        "closure reconciliation",
    )
    metrics = mapping(closure["review_state_metrics"], "closure review-state metrics")
    expected_metrics = {
        "resolved_required_interaction": review_metrics.get("resolved_required_interaction", 0),
        "resolved_not_an_interaction_with_proof": review_metrics.get(
            "resolved_not_an_interaction_with_proof", 0
        ),
        "resolved_out_of_declared_scope_with_reason": review_metrics.get(
            "resolved_out_of_declared_scope_with_reason", 0
        ),
        "unresolved": review_metrics.get("unresolved", 0),
    }
    require(metrics == expected_metrics, "AGGREGATE_COUNT_MISMATCH", "closure review-state metrics")
    if metrics["unresolved"] and gate_status == EXPECTED_PASS_GATE_STATUS:
        fail(
            "UNRESOLVED_CANDIDATE_ON_PASS",
            "closure claims PASS while unresolved candidates remain",
        )
    expected_gate = EXPECTED_GATE_STATUS if metrics["unresolved"] else EXPECTED_PASS_GATE_STATUS
    require(
        gate_status == expected_gate,
        "DOWNSTREAM_STATUS_PROMOTED",
        "closure gate vocabulary/status",
    )
    require(
        mapping(closure["flags"], "closure flags") == EXPECTED_FLAGS,
        "DOWNSTREAM_STATUS_PROMOTED",
        "closure flags",
    )


def validate_matrix(matrix: dict[str, Any]) -> None:
    exact_keys(matrix, {"schema", "model_id", "cases"}, "negative matrix")
    require(
        matrix["schema"] == C_JSON_SCHEMAS[MATRIX_NAME], "SCHEMA_MISMATCH", "negative matrix schema"
    )
    actual = list_value(matrix["cases"], "negative matrix cases")
    expected = negative_matrix()["cases"]
    require(
        actual == expected,
        "NEGATIVE_MATRIX_INVALID",
        "C negative matrix is not exactly C-001 through C-042",
    )


def expected_summary_artifact_names() -> set[str]:
    return {
        "C_DESIGN_SPEC.md",
        MODEL_NAME,
        REVIEW_NAME,
        UNIVERSE_NAME,
        CLASSES_NAME,
        CLASSIFICATIONS_NAME,
        *SHARD_NAMES,
        CLOSURE_NAME,
        REPORT_NAME,
        MATRIX_NAME,
    }


def summary_artifact_raw(name: str) -> bytes:
    return local_raw(f"verification/{name}" if name == MATRIX_NAME else name)


def validate_migration_parity_summary(summary: dict[str, Any]) -> None:
    value = mapping(summary["migration_parity"], "migration parity")
    exact_keys(
        value,
        {
            "source_execution_commit",
            "source_classifications_path",
            "source_classifications_raw_sha256",
            "source_upstream_artifact_digests",
            "target_classification_root_path",
            "target_shard_paths",
            "source_record_count",
            "target_record_count",
            "record_equality",
            "ordering_transform",
            "result",
        },
        "migration parity",
    )
    require(
        value["source_execution_commit"] == MIGRATION_SOURCE_EXECUTION_COMMIT,
        "MIGRATION_PARITY_INVALID",
        "migration source execution commit",
    )
    require(
        value["source_classifications_path"] == MIGRATION_SOURCE_CLASSIFICATIONS_PATH,
        "MIGRATION_PARITY_INVALID",
        "migration source classifications path",
    )
    require(
        value["target_classification_root_path"] == CLASSIFICATIONS_NAME,
        "MIGRATION_PARITY_INVALID",
        "migration target root path",
    )
    require(
        value["target_shard_paths"] == list(SHARD_NAMES),
        "MIGRATION_PARITY_INVALID",
        "migration target shard paths",
    )
    require(
        value["ordering_transform"] == PARTITION_SCHEME,
        "MIGRATION_PARITY_INVALID",
        "migration ordering transform",
    )
    if value["result"] == "NOT_RUN":
        return
    require(
        value["result"] == "PASS"
        and value["record_equality"] == "PASS"
        and value["source_record_count"] == EXPECTED_REV3_CANDIDATE_COUNT
        and value["target_record_count"] == EXPECTED_REV3_CANDIDATE_COUNT,
        "MIGRATION_PARITY_INVALID",
        "migration parity did not pass for the complete candidate set",
    )
    hex64(value["source_classifications_raw_sha256"], "migration source classifications digest")
    source_upstreams = mapping(
        value["source_upstream_artifact_digests"], "migration source upstream digests"
    )
    expected_paths = {
        f"sources/m2_5/closures/C/{name}": sha256_bytes(summary_artifact_raw(name))
        for name in (MODEL_NAME, REVIEW_NAME, UNIVERSE_NAME, CLASSES_NAME)
    }
    require(
        source_upstreams == expected_paths,
        "MIGRATION_PARITY_INVALID",
        "migration upstream digest set",
    )


def validate_publishability_summary(summary: dict[str, Any]) -> None:
    value = mapping(summary["publishability_preflight"], "publishability preflight")
    exact_keys(
        value,
        {
            "base_ref",
            "verified_base_commit",
            "checked_commit",
            "introduced_object_count",
            "introduced_blob_count",
            "hosting_hard_limit_bytes",
            "oversized_blobs",
            "forbidden_legacy_paths",
            "result",
        },
        "publishability preflight",
    )
    require(
        value["base_ref"] == "origin/master"
        and value["verified_base_commit"] == EXPECTED_PREVIOUS_MASTER
        and value["hosting_hard_limit_bytes"] == HOSTING_HARD_LIMIT_BYTES,
        "PUBLISHABILITY_PRECHECK_INVALID",
        "publishability base identity",
    )
    if summary["execution_commit"] is None and value["result"] == "NOT_RUN":
        return
    execution_commit = string(summary["execution_commit"], "summary execution commit")
    require(
        value["result"] == "PASS"
        and value["checked_commit"] == execution_commit
        and isinstance(value["introduced_object_count"], int)
        and isinstance(value["introduced_blob_count"], int)
        and value["introduced_object_count"] >= value["introduced_blob_count"] >= 0
        and value["oversized_blobs"] == []
        and value["forbidden_legacy_paths"] == [],
        "PUBLISHABILITY_PRECHECK_INVALID",
        "publishability preflight did not pass cleanly",
    )


def validate_summary(snapshot: Snapshot) -> None:
    summary = get_artifact(snapshot, SUMMARY_NAME)
    exact_keys(
        summary,
        {
            "schema",
            "execution_commit",
            "source_tree_before_fingerprint",
            "source_tree_after_fingerprint",
            "prerequisite_results",
            "c_result",
            "negative_test_result",
            "repository_gate_results",
            "artifact_digests",
            "migration_parity",
            "publishability_preflight",
            "checker_identities",
            "evidence_protocol",
            "evidence_export",
        },
        "verification summary",
    )
    require(
        summary["schema"] == C_JSON_SCHEMAS[SUMMARY_NAME],
        "SCHEMA_MISMATCH",
        "verification summary schema",
    )
    validate_migration_parity_summary(summary)
    validate_publishability_summary(summary)
    identities = mapping(summary["checker_identities"], "checker identities")
    exact_keys(identities, {"c_checker", "master_drift_checker"}, "checker identities")
    for key, path in (
        ("c_checker", CHECKER_RELATIVE_PATH),
        ("master_drift_checker", "scripts/check_m2_5_master_drift.py"),
    ):
        item = mapping(identities[key], f"checker identities.{key}")
        exact_keys(item, {"path", "raw_sha256"}, f"checker identities.{key}")
        require(item["path"] == path, "CHECKER_IDENTITY_MISMATCH", f"checker identities.{key}.path")
        expected_path = ROOT / path
        require(
            expected_path.is_file()
            and item["raw_sha256"] == sha256_bytes(expected_path.read_bytes()),
            "CHECKER_IDENTITY_MISMATCH",
            f"checker identities.{key}.raw_sha256",
        )
    evidence = mapping(summary["evidence_protocol"], "evidence protocol")
    exact_keys(evidence, {"H_exec", "modified_path", "H_evidence_relation"}, "evidence protocol")
    require(
        evidence["modified_path"]
        == "sources/m2_5/closures/C/verification/c_verification_summary.v3.json",
        "SOURCE_CHANGED_AFTER_H_EXEC",
        "evidence modified path",
    )
    require(
        evidence["H_evidence_relation"] == "direct_child_summary_only",
        "SOURCE_CHANGED_AFTER_H_EXEC",
        "evidence relation",
    )
    digests = mapping(summary["artifact_digests"], "summary artifact digests")
    expected_names = expected_summary_artifact_names()
    require(
        set(digests) in (set(), expected_names),
        "EVIDENCE_DIGEST_BINDING_MISMATCH",
        "summary artifact digest inventory",
    )
    for name, digest in digests.items():
        require(
            digest == sha256_bytes(summary_artifact_raw(name)),
            "EVIDENCE_DIGEST_BINDING_MISMATCH",
            f"summary digest {name}",
        )
    export = mapping(summary["evidence_export"], "verification summary.evidence_export")
    exact_keys(export, {"status", "path", "sha256"}, "verification summary.evidence_export")
    if summary["execution_commit"] is None:
        require(
            export == {"status": "NOT_RUN", "path": None, "sha256": None},
            "SOURCE_CHANGED_AFTER_H_EXEC",
            "provisional evidence export",
        )
        return
    if not re.fullmatch(r"[0-9a-f]{40}", string(summary["execution_commit"], "execution_commit")):
        fail("SOURCE_CHANGED_AFTER_H_EXEC", "execution_commit is not a Git SHA")
    for key in ("prerequisite_results", "negative_test_result", "repository_gate_results"):
        section = mapping(summary[key], f"verification summary.{key}")
        require(
            section.get("status") == "PASS",
            "SOURCE_CHANGED_AFTER_H_EXEC",
            f"{key} is not PASS",
        )
    c_result = mapping(summary["c_result"], "verification summary.c_result")
    require(
        c_result.get("status") in {"PASS", "BLOCKED"},
        "SOURCE_CHANGED_AFTER_H_EXEC",
        "C result is neither PASS nor BLOCKED",
    )
    closure = get_artifact(snapshot, CLOSURE_NAME)
    unresolved = mapping(closure["review_state_metrics"], "closure review-state metrics")[
        "unresolved"
    ]
    require(
        c_result["status"] == ("BLOCKED" if unresolved else "PASS"),
        "SOURCE_CHANGED_AFTER_H_EXEC",
        "summary C result disagrees with closure review state",
    )
    require(
        summary["negative_test_result"].get("case_count") == 42,
        "SOURCE_CHANGED_AFTER_H_EXEC",
        "summary negative case count",
    )
    execution_commit = string(summary["execution_commit"], "execution_commit")
    fingerprint = tracked_tree_fingerprint(execution_commit)
    before = hex64(summary["source_tree_before_fingerprint"], "source_tree_before_fingerprint")
    after = hex64(summary["source_tree_after_fingerprint"], "source_tree_after_fingerprint")
    require(
        before == fingerprint and after == fingerprint,
        "SOURCE_CHANGED_AFTER_H_EXEC",
        "source-tree fingerprint does not match H_exec",
    )
    require(export["status"] == "PASS", "SOURCE_CHANGED_AFTER_H_EXEC", "review export is not PASS")
    export_path = Path(nonempty_string(export["path"], "evidence export path")).resolve()
    try:
        export_path.relative_to(ROOT)
    except ValueError:
        pass
    else:
        fail("SOURCE_CHANGED_AFTER_H_EXEC", "review export must be outside the repository")
    require(
        export_path.is_file()
        and sha256_bytes(export_path.read_bytes())
        == hex64(export["sha256"], "evidence export sha256"),
        "SOURCE_CHANGED_AFTER_H_EXEC",
        "review export is missing or has a different digest",
    )
    actual_publishability = publishability_preflight(
        execution_commit, EXPECTED_PREVIOUS_MASTER
    )
    require(
        actual_publishability == summary["publishability_preflight"],
        "PUBLISHABILITY_PRECHECK_INVALID",
        "persisted H_exec publishability evidence differs from the executed scan",
    )
    validate_historical_evidence_chain(summary, snapshot.raw[SUMMARY_NAME])


def finalize_summary(execution_commit: str, results_path: Path, export_path: Path) -> None:
    """Write the one permitted post-H_exec summary projection."""
    if not re.fullmatch(r"[0-9a-f]{40}", execution_commit):
        blocked("EVIDENCE_EXECUTION_SHA_INVALID", f"invalid H_exec {execution_commit!r}")
    if not export_path.is_file():
        blocked("EVIDENCE_EXPORT_UNAVAILABLE", f"review export not found: {export_path}")
    try:
        export_path.resolve().relative_to(ROOT)
    except ValueError:
        pass
    else:
        fail("EVIDENCE_EXPORT_IN_REPOSITORY", "review export must remain outside the repository")
    try:
        results = parse_json(results_path.read_bytes(), "execution results")
    except OSError as exc:
        blocked("EVIDENCE_RESULTS_UNAVAILABLE", str(exc))
    result = mapping(results, "execution results")
    exact_keys(
        result,
        {
            "prerequisite_results",
            "c_result",
            "negative_test_result",
            "repository_gate_results",
            "migration_parity",
            "publishability_preflight",
        },
        "execution results",
    )
    for key in ("prerequisite_results", "negative_test_result", "repository_gate_results"):
        section = mapping(result[key], f"execution results.{key}")
        if section.get("status") != "PASS":
            fail("EVIDENCE_RESULTS_NOT_PASS", f"execution results.{key} is not PASS")
    c_result = mapping(result["c_result"], "execution results.c_result")
    if c_result.get("status") not in {"PASS", "BLOCKED"}:
        fail("EVIDENCE_RESULTS_NOT_PASS", "execution results.c_result is neither PASS nor BLOCKED")
    negative = mapping(result["negative_test_result"], "execution results.negative_test_result")
    require(
        negative.get("case_count") == 42,
        "EVIDENCE_RESULTS_NOT_PASS",
        "execution results do not contain all 42 negative cases",
    )
    snapshot = load_snapshot()
    summary = get_artifact(snapshot, SUMMARY_NAME)
    summary["execution_commit"] = execution_commit
    fingerprint = tracked_tree_fingerprint(execution_commit)
    summary["source_tree_before_fingerprint"] = fingerprint
    summary["source_tree_after_fingerprint"] = fingerprint
    summary["prerequisite_results"] = result["prerequisite_results"]
    summary["c_result"] = result["c_result"]
    summary["negative_test_result"] = result["negative_test_result"]
    summary["repository_gate_results"] = result["repository_gate_results"]
    summary["migration_parity"] = result["migration_parity"]
    summary["publishability_preflight"] = result["publishability_preflight"]
    summary["artifact_digests"] = {
        name: sha256_bytes(summary_artifact_raw(name)) for name in expected_summary_artifact_names()
    }
    summary["checker_identities"] = {
        "c_checker": {
            "path": CHECKER_RELATIVE_PATH,
            "raw_sha256": sha256_bytes(Path(__file__).read_bytes()),
        },
        "master_drift_checker": {
            "path": "scripts/check_m2_5_master_drift.py",
            "raw_sha256": sha256_bytes((ROOT / "scripts/check_m2_5_master_drift.py").read_bytes()),
        },
    }
    summary["evidence_protocol"]["H_exec"] = execution_commit
    summary["evidence_export"] = {
        "status": "PASS",
        "path": str(export_path.resolve()),
        "sha256": sha256_bytes(export_path.read_bytes()),
    }
    (C_DIR / "verification" / SUMMARY_NAME).write_bytes(json_bytes(summary))
    print(
        f"FINALIZED_SUMMARY H_exec={execution_commit} "
        f"export_sha256={summary['evidence_export']['sha256']}"
    )


def validate_snapshot(snapshot: Snapshot, run_prereqs: bool = True) -> None:
    if run_prereqs:
        validate_prerequisites()
    reader = load_archive()
    model = get_artifact(snapshot, MODEL_NAME)
    review = get_artifact(snapshot, REVIEW_NAME)
    classes = get_artifact(snapshot, CLASSES_NAME)
    classifications_root = get_artifact(snapshot, CLASSIFICATIONS_NAME)
    closure = get_artifact(snapshot, CLOSURE_NAME)
    matrix = get_artifact(snapshot, MATRIX_NAME)
    model_raw = snapshot.raw[MODEL_NAME]
    review_raw = snapshot.raw[REVIEW_NAME]
    validate_model(model)
    validate_review(review, model_raw)
    catalog = mapping(
        parse_json(
            (ROOT / "sources/m2_5/closures/B2/requirement_family_catalog.v1.json").read_bytes(),
            "B2 catalog",
        ),
        "B2 catalog",
    )
    b2 = mapping(
        parse_json(
            (ROOT / "sources/m2_5/closures/B2/card_semantic_classifications.v1.json").read_bytes(),
            "B2 classifications",
        ),
        "B2 classifications",
    )
    b1 = mapping(
        parse_json(
            (ROOT / "sources/m2_5/closures/B1/official_authority_citations.v3.json").read_bytes(),
            "B1.Final citations",
        ),
        "B1.Final citations",
    )
    validate_external_prereqs(reader, catalog, b2, b1)
    validate_closure_early(closure, snapshot, reader)
    expected, instance_lookup = validate_universe(snapshot, reader, catalog, model_raw, review_raw)
    classes_by_id = validate_classes(snapshot, classes, catalog, b1, b2_classification_map())
    classifications = validate_classification_bundle(
        snapshot, classifications_root, expected
    )
    terminal = validate_classifications(
        snapshot, classifications, expected, instance_lookup, classes_by_id, catalog, b1
    )
    validate_closure(
        snapshot,
        closure,
        model_raw,
        review_raw,
        snapshot.raw[UNIVERSE_NAME],
        snapshot.raw[CLASSES_NAME],
        snapshot.raw[CLASSIFICATIONS_NAME],
        terminal,
        len(expected),
        len(classes_by_id),
    )
    validate_matrix(matrix)
    validate_summary(snapshot)


def mutation_cases(snapshot: Snapshot) -> dict[str, Callable[[], None]]:
    cases: dict[str, Callable[[], None]] = {}

    def reverse_directional(candidate_universe: dict[str, Any]) -> None:
        candidate = next(
            c
            for c in candidate_universe["candidates"]
            if c["relation"] == "directional_binary"
            and c["participant_refs"][0] != c["participant_refs"][1]
        )
        candidate["participant_refs"] = list(reversed(candidate["participant_refs"]))

    def targeted_review_snapshot(review_id: str, include_other_record: bool) -> Snapshot:
        copy_snapshot = snapshot.clone()
        if include_other_record:
            review = get_artifact(copy_snapshot, REVIEW_NAME)
            review["review_records"] = [
                {
                    "review_record_id": "ira.v1/other",
                    "review_kind": "targeted_higher_order_review",
                    "participant_source_refs": [],
                    "review_evidence_refs": [],
                    "review_rationale": "Synthetic negative-test record only.",
                }
            ]
            review["review_record_count"] = 1
            set_artifact(copy_snapshot, REVIEW_NAME, review)
        universe = get_artifact(copy_snapshot, UNIVERSE_NAME)
        candidate = universe["candidates"][0]
        candidate["source_origin"] = "targeted_higher_order_review"
        candidate["source_binding"] = {
            "kind": "targeted_higher_order_review",
            "additions_path": "sources/m2_5/closures/C/interaction_review_additions.v2.json",
            "additions_raw_sha256": sha256_bytes(copy_snapshot.raw[REVIEW_NAME]),
            "review_record_id": review_id,
            "review_kind": "targeted_higher_order_review",
            "participant_source_refs": [],
            "review_evidence_refs": [],
        }
        universe["input_bindings"]["review_additions"]["raw_sha256"] = sha256_bytes(
            copy_snapshot.raw[REVIEW_NAME]
        )
        set_artifact(copy_snapshot, UNIVERSE_NAME, universe)
        return copy_snapshot

    def mutate(name: str, fn: Callable[[dict[str, Any]], None]) -> Snapshot:
        copy_snapshot = snapshot.clone()
        value = get_artifact(copy_snapshot, name)
        fn(value)
        set_artifact(copy_snapshot, name, value)
        return copy_snapshot

    def mutate_classification(
        selector: Callable[[dict[str, Any]], bool],
        mutation: Callable[[dict[str, Any]], None],
    ) -> Snapshot:
        copy_snapshot = snapshot.clone()
        for relative in SHARD_NAMES:
            shard = get_artifact(copy_snapshot, relative)
            for raw_record in shard["candidate_classifications"]:
                record = mapping(raw_record, "classification mutation record")
                if selector(record):
                    mutation(record)
                    set_artifact(copy_snapshot, relative, shard)
                    root = get_artifact(copy_snapshot, CLASSIFICATIONS_NAME)
                    root["shards"][SHARD_NAMES.index(relative)]["raw_sha256"] = sha256_bytes(
                        copy_snapshot.raw[relative]
                    )
                    set_artifact(copy_snapshot, CLASSIFICATIONS_NAME, root)
                    return copy_snapshot
        raise AssertionError("classification mutation selector found no record")

    cases["C-001"] = lambda: validate_snapshot(
        mutate(
            CLOSURE_NAME,
            lambda x: mapping(x["gate_status"], "gate").__setitem__(
                "CLASSIFICATION_REFERENCE_CLOSURE", "BLOCKED"
            ),
        ),
        False,
    )
    cases["C-002"] = lambda: validate_snapshot(
        mutate(
            UNIVERSE_NAME,
            lambda x: mapping(x["input_bindings"]["b2_artifacts"][0], "b2").__setitem__(
                "raw_sha256", "0" * 64
            ),
        ),
        False,
    )
    cases["C-003"] = lambda: validate_snapshot(
        mutate(
            UNIVERSE_NAME,
            lambda x: mapping(x["input_bindings"]["b2_artifacts"][1], "b2").__setitem__(
                "raw_sha256", "0" * 64
            ),
        ),
        False,
    )
    cases["C-004"] = lambda: validate_snapshot(
        mutate(
            CLASSES_NAME,
            lambda x: mapping(x["classes"][0]["b2_boundary_refs"][0], "boundary").__setitem__(
                "precise_semantic_definition", "tampered"
            ),
        ),
        False,
    )
    cases["C-005"] = lambda: validate_snapshot(
        mutate(
            CLOSURE_NAME,
            lambda x: mapping(
                x["external_prerequisite_identities"]["b1_final"][0], "b1"
            ).__setitem__("raw_sha256", "0" * 64),
        ),
        False,
    )
    cases["C-006"] = lambda: validate_snapshot(
        mutate(
            CLOSURE_NAME,
            lambda x: mapping(
                x["external_prerequisite_identities"]["rev3_archive"], "archive"
            ).__setitem__("archive_member_sha256", "0" * 64),
        ),
        False,
    )
    cases["C-007"] = lambda: validate_snapshot(
        mutate(UNIVERSE_NAME, lambda x: x["candidates"].pop()), False
    )
    cases["C-008"] = lambda: validate_snapshot(
        mutate(
            CLOSURE_NAME,
            lambda x: x["gate_status"].__setitem__("DECLARED_INTERACTION_MODEL_CLOSURE", "PASS"),
        ),
        False,
    )
    cases["C-009"] = lambda: validate_snapshot(
        mutate(
            UNIVERSE_NAME,
            lambda x: x["candidates"][-1]["participant_refs"][0].__setitem__(
                "semantic_ref", "00000000-0000-0000-0000-000000000000"
            ),
        ),
        False,
    )
    cases["C-010"] = lambda: validate_snapshot(
        mutate(
            UNIVERSE_NAME,
            lambda x: next(c for c in x["candidates"] if c["relation"] == "unordered_binary")[
                "participant_refs"
            ][0].__setitem__("semantic_ref", "cap.unknown"),
        ),
        False,
    )
    cases["C-011"] = lambda: validate_snapshot(
        mutate_classification(
            lambda c: c["terminal_disposition"] == "required_interaction",
            lambda c: c["source_instance_context_mappings"][0]["b2_assignment_refs"][0].__setitem__(
                "assignment_ordinal", -1
            ),
        ),
        False,
    )
    cases["C-012"] = lambda: validate_snapshot(
        mutate(
            CLASSES_NAME,
            lambda x: x["classes"][0]["b2_family_refs"][0].__setitem__(
                "lifecycle", "active_unassigned"
            ),
        ),
        False,
    )
    cases["C-013"] = lambda: validate_snapshot(
        mutate(
            CLASSES_NAME,
            lambda x: x["classes"][1].__setitem__(
                "interaction_class_id", x["classes"][0]["interaction_class_id"]
            ),
        ),
        False,
    )
    cases["C-014"] = lambda: validate_snapshot(
        mutate_classification(
            lambda _c: True,
            lambda c: c["source_instance_context_mappings"].append(
                copy.deepcopy(c["source_instance_context_mappings"][0])
            ),
        ),
        False,
    )
    cases["C-015"] = lambda: validate_snapshot(
        mutate(
            UNIVERSE_NAME,
            lambda x: x["source_instances"].append(
                copy.deepcopy(x["source_instances"][0])
                | {"source_instance_id": "si.v1/orphan/0", "candidate_id": "orphan"}
            ),
        ),
        False,
    )
    cases["C-016"] = lambda: validate_snapshot(mutate(UNIVERSE_NAME, reverse_directional), False)
    cases["C-017"] = lambda: validate_snapshot(
        mutate(
            UNIVERSE_NAME,
            lambda x: next(
                c for c in x["candidates"] if c["relation"] == "directional_binary"
            ).__setitem__("relation", "unordered_binary"),
        ),
        False,
    )
    cases["C-018"] = lambda: validate_snapshot(
        mutate(CLASSES_NAME, lambda x: x["classes"][0]["participant_roles"].pop()), False
    )
    cases["C-019"] = lambda: validate_snapshot(
        mutate(
            CLASSES_NAME,
            lambda x: x["classes"][0].update(
                {
                    "arity": "higher_order",
                    "participant_roles": [copy.deepcopy(x["classes"][0]["participant_roles"][0])],
                }
            ),
        ),
        False,
    )
    cases["C-020"] = lambda: validate_snapshot(
        mutate(
            CLASSES_NAME, lambda x: x["classes"][0].__setitem__("host_relationship", "cross_host")
        ),
        False,
    )
    cases["C-021"] = lambda: validate_snapshot(
        mutate(
            CLASSES_NAME, lambda x: x["classes"][0].__setitem__("host_relationship", "same_host")
        ),
        False,
    )
    cases["C-022"] = lambda: validate_snapshot(
        mutate(CLASSES_NAME, lambda x: x["classes"][0]["context_dimensions"].pop("zone")), False
    )
    cases["C-023"] = lambda: validate_snapshot(
        mutate(CLASSES_NAME, lambda x: x["classes"][0]["b1_final_citation_refs"].clear()), False
    )
    cases["C-024"] = lambda: validate_snapshot(
        mutate(
            CLASSES_NAME,
            lambda x: x["classes"][0]["source_evidence_refs"].append(
                {"authority_kind": "unknown", "path": "x", "locator": {}, "raw_sha256": "0" * 64}
            ),
        ),
        False,
    )
    cases["C-025"] = lambda: validate_snapshot(
        mutate(
            CLOSURE_NAME,
            lambda x: mapping(x["external_prerequisite_identities"]["b2"][0], "b2").__setitem__(
                "raw_sha256", "1" * 64
            ),
        ),
        False,
    )
    cases["C-026"] = lambda: validate_snapshot(
        mutate(
            CLOSURE_NAME, lambda x: x["candidate_reconciliation"].__setitem__("current_total", 1)
        ),
        False,
    )
    cases["C-027"] = lambda: validate_snapshot(
        mutate_classification(
            lambda _c: True,
            lambda c: c.__setitem__("terminal_disposition", "pending"),
        ),
        False,
    )
    cases["C-028"] = lambda: validate_snapshot(
        mutate(
            CLOSURE_NAME,
            lambda x: x["gate_status"].__setitem__("REV2_REUSE_RATIO_REPRODUCIBLE", "PASS"),
        ),
        False,
    )
    cases["C-029"] = lambda: validate_snapshot(
        mutate(CLOSURE_NAME, lambda x: x["flags"].__setitem__("DECK_PAIR_LOCKED", True)), False
    )
    cases["C-030"] = lambda: validate_snapshot(
        mutate(CLOSURE_NAME, lambda x: x["flags"].__setitem__("M3_STARTED", True)), False
    )
    cases["C-031"] = lambda: validate_snapshot(
        mutate(
            SUMMARY_NAME, lambda x: x["evidence_protocol"].__setitem__("modified_path", "wrong")
        ),
        False,
    )
    cases["C-032"] = lambda: validate_snapshot(
        mutate(SUMMARY_NAME, lambda x: x["artifact_digests"].__setitem__(MODEL_NAME, "0" * 64)),
        False,
    )
    cases["C-033"] = lambda: validate_snapshot(
        mutate(
            UNIVERSE_NAME,
            lambda x: next(
                c for c in x["candidates"] if c["relation"] == "unordered_binary"
            ).__setitem__("relation", "UNORDERED_BINARY"),
        ),
        False,
    )
    cases["C-034"] = lambda: validate_snapshot(
        mutate(MODEL_NAME, lambda x: x["participant_kind_vocabulary"].append("future_kind")), False
    )
    cases["C-035"] = lambda: validate_snapshot(
        targeted_review_snapshot("ira.v1/missing", False), False
    )
    cases["C-036"] = lambda: validate_snapshot(
        targeted_review_snapshot("ira.v1/unknown", True), False
    )
    cases["C-037"] = lambda: validate_snapshot(
        mutate(
            UNIVERSE_NAME,
            lambda x: x["input_bindings"]["review_additions"].__setitem__("raw_sha256", "0" * 64),
        ),
        False,
    )
    cases["C-038"] = lambda: validate_snapshot(
        mutate(
            UNIVERSE_NAME,
            lambda x: x["candidates"][0]["source_binding"].__setitem__("kind", "b2_derived"),
        ),
        False,
    )
    cases["C-039"] = lambda: validate_snapshot(
        mutate(
            UNIVERSE_NAME,
            lambda x: x["candidates"][0]["candidate_identity"].__setitem__("digest_hex", "0" * 64),
        ),
        False,
    )
    cases["C-040"] = lambda: validate_snapshot(
        mutate(
            SUMMARY_NAME,
            lambda x: mapping(x["checker_identities"]["c_checker"], "checker").__setitem__(
                "raw_sha256", "0" * 64
            ),
        ),
        False,
    )
    cases["C-041"] = lambda: validate_snapshot(
        mutate(
            UNIVERSE_NAME,
            lambda x: x["source_instances"].append(copy.deepcopy(x["source_instances"][0])),
        ),
        False,
    )
    cases["C-042"] = lambda: validate_snapshot(
        mutate(
            UNIVERSE_NAME,
            lambda x: x["candidates"][0]["source_binding"].__setitem__(
                "archive_member_sha256", "0" * 63
            ),
        ),
        False,
    )
    return cases


def classification_layout_regression_cases(snapshot: Snapshot) -> dict[str, Callable[[], None]]:
    """Return focused V3 root/shard mutations outside the fixed C-001..C-042 matrix."""
    universe = get_artifact(snapshot, UNIVERSE_NAME)
    expected_candidates = [
        mapping(candidate, "layout regression candidate")
        for candidate in list_value(universe["candidates"], "layout regression candidates")
    ]

    def mutated_snapshot(
        relative: str, mutation: Callable[[dict[str, Any]], None], update_root_digest: bool = True
    ) -> Snapshot:
        copy_snapshot = snapshot.clone()
        shard = get_artifact(copy_snapshot, relative)
        mutation(shard)
        set_artifact(copy_snapshot, relative, shard)
        if update_root_digest:
            root = get_artifact(copy_snapshot, CLASSIFICATIONS_NAME)
            root["shards"][SHARD_NAMES.index(relative)]["raw_sha256"] = sha256_bytes(
                copy_snapshot.raw[relative]
            )
            set_artifact(copy_snapshot, CLASSIFICATIONS_NAME, root)
        return copy_snapshot

    def validate_bundle(copy_snapshot: Snapshot) -> None:
        validate_classification_bundle(
            copy_snapshot,
            get_artifact(copy_snapshot, CLASSIFICATIONS_NAME),
            expected_candidates,
        )

    def missing_shard() -> None:
        copy_snapshot = snapshot.clone()
        relative = SHARD_NAMES[0]
        del copy_snapshot.values[relative]
        del copy_snapshot.raw[relative]
        validate_bundle(copy_snapshot)

    def extra_shard() -> None:
        actual = set(EXPECTED_C_DIRECTORY_FILES)
        actual.add(f"{SHARD_DIRECTORY}/interaction_classifications.0016.v3.json")
        validate_c_file_inventory(actual)

    def missing_root() -> None:
        actual = set(EXPECTED_C_DIRECTORY_FILES)
        actual.remove(CLASSIFICATIONS_NAME)
        validate_c_file_inventory(actual)

    def extra_root_entry() -> None:
        copy_snapshot = snapshot.clone()
        root = get_artifact(copy_snapshot, CLASSIFICATIONS_NAME)
        root["shards"].append(copy.deepcopy(root["shards"][0]))
        set_artifact(copy_snapshot, CLASSIFICATIONS_NAME, root)
        validate_bundle(copy_snapshot)

    def root_count_mismatch() -> None:
        copy_snapshot = snapshot.clone()
        root = get_artifact(copy_snapshot, CLASSIFICATIONS_NAME)
        root["classification_count"] -= 1
        set_artifact(copy_snapshot, CLASSIFICATIONS_NAME, root)
        validate_bundle(copy_snapshot)

    def root_records_forbidden() -> None:
        copy_snapshot = snapshot.clone()
        root = get_artifact(copy_snapshot, CLASSIFICATIONS_NAME)
        root["candidate_classifications"] = []
        set_artifact(copy_snapshot, CLASSIFICATIONS_NAME, root)
        validate_bundle(copy_snapshot)

    def shard_root_digest_forbidden() -> None:
        copy_snapshot = snapshot.clone()
        shard = get_artifact(copy_snapshot, SHARD_NAMES[0])
        shard["classification_root_raw_sha256"] = "0" * 64
        set_artifact(copy_snapshot, SHARD_NAMES[0], shard)
        validate_bundle(copy_snapshot)

    def wrong_shard_index() -> None:
        copy_snapshot = snapshot.clone()
        root = get_artifact(copy_snapshot, CLASSIFICATIONS_NAME)
        root["shards"][0]["shard_index"] = 1
        set_artifact(copy_snapshot, CLASSIFICATIONS_NAME, root)
        validate_bundle(copy_snapshot)

    def wrong_shard_path() -> None:
        copy_snapshot = snapshot.clone()
        root = get_artifact(copy_snapshot, CLASSIFICATIONS_NAME)
        root["shards"][0]["path"] = SHARD_NAMES[1]
        set_artifact(copy_snapshot, CLASSIFICATIONS_NAME, root)
        validate_bundle(copy_snapshot)

    def wrong_shard_digest() -> None:
        copy_snapshot = snapshot.clone()
        root = get_artifact(copy_snapshot, CLASSIFICATIONS_NAME)
        root["shards"][0]["raw_sha256"] = "0" * 64
        set_artifact(copy_snapshot, CLASSIFICATIONS_NAME, root)
        validate_bundle(copy_snapshot)

    def wrong_record_count() -> None:
        validate_bundle(
            mutated_snapshot(
                SHARD_NAMES[0], lambda shard: shard.__setitem__("record_count", 999)
            )
        )

    def wrong_shard_candidate() -> None:
        copy_snapshot = snapshot.clone()
        first = get_artifact(copy_snapshot, SHARD_NAMES[0])
        second = get_artifact(copy_snapshot, SHARD_NAMES[1])
        first["candidate_classifications"][0], second["candidate_classifications"][0] = (
            second["candidate_classifications"][0],
            first["candidate_classifications"][0],
        )
        set_artifact(copy_snapshot, SHARD_NAMES[0], first)
        set_artifact(copy_snapshot, SHARD_NAMES[1], second)
        root = get_artifact(copy_snapshot, CLASSIFICATIONS_NAME)
        for index in (0, 1):
            root["shards"][index]["raw_sha256"] = sha256_bytes(
                copy_snapshot.raw[SHARD_NAMES[index]]
            )
        set_artifact(copy_snapshot, CLASSIFICATIONS_NAME, root)
        validate_bundle(copy_snapshot)

    def duplicate_candidate() -> None:
        copy_snapshot = mutated_snapshot(
            SHARD_NAMES[1],
            lambda shard: shard["candidate_classifications"].__setitem__(
                0,
                copy.deepcopy(
                    get_artifact(snapshot, SHARD_NAMES[0])["candidate_classifications"][0]
                ),
            ),
        )
        validate_bundle(copy_snapshot)

    def missing_candidate() -> None:
        copy_snapshot = mutated_snapshot(
            SHARD_NAMES[-1], lambda shard: shard["candidate_classifications"].pop()
        )
        validate_bundle(copy_snapshot)

    def reordered_partition() -> None:
        def swap_first_two(shard: dict[str, Any]) -> None:
            records = shard["candidate_classifications"]
            records[0], records[1] = records[1], records[0]

        copy_snapshot = mutated_snapshot(SHARD_NAMES[0], swap_first_two)
        validate_bundle(copy_snapshot)

    def oversized_shard() -> None:
        validate_shard_size(SHARD_MAX_BYTES + 1, 0)

    return {
        "missing_shard": missing_shard,
        "extra_shard": extra_shard,
        "missing_root": missing_root,
        "extra_root_entry": extra_root_entry,
        "root_count_mismatch": root_count_mismatch,
        "root_records_forbidden": root_records_forbidden,
        "shard_root_digest_forbidden": shard_root_digest_forbidden,
        "wrong_shard_index": wrong_shard_index,
        "wrong_shard_path": wrong_shard_path,
        "wrong_shard_digest": wrong_shard_digest,
        "wrong_record_count": wrong_record_count,
        "candidate_in_wrong_shard": wrong_shard_candidate,
        "duplicate_candidate_across_shards": duplicate_candidate,
        "missing_candidate_across_shards": missing_candidate,
        "non_contiguous_reordered_partition": reordered_partition,
        "shard_over_50_mib": oversized_shard,
    }


def publishability_history_regression() -> None:
    """Prove that deleted oversized blobs fail, while unrelated refs are ignored."""
    with tempfile.TemporaryDirectory(prefix="m2-5-c-publishability-") as temporary:
        repo = Path(temporary)

        def run_git(*args: str) -> str:
            result = subprocess.run(
                ["git", "-C", str(repo), *args],
                check=True,
                capture_output=True,
                text=True,
            )
            return result.stdout.strip()

        run_git("init", "--quiet")
        run_git("config", "user.email", "m2-5-c@example.invalid")
        run_git("config", "user.name", "M2.5.C regression")
        (repo / "base.txt").write_text("base\n", encoding="utf-8")
        run_git("add", "base.txt")
        run_git("commit", "--quiet", "-m", "base")
        base = run_git("rev-parse", "HEAD")
        run_git("branch", "-M", "main")

        oversized_path = repo / "oversized.bin"
        oversized_path.write_bytes(b"x" * (HOSTING_HARD_LIMIT_BYTES + 1))
        run_git("add", "oversized.bin")
        run_git("commit", "--quiet", "-m", "introduce oversized blob")
        run_git("rev-parse", "HEAD")
        oversized_path.unlink()
        run_git("add", "-u")
        run_git("commit", "--quiet", "-m", "delete oversized blob")
        checked_with_deleted_blob = run_git("rev-parse", "HEAD")
        run_git("branch", "unrelated", checked_with_deleted_blob)

        run_git("checkout", "--quiet", "-b", "safe", base)
        (repo / "safe.txt").write_text("safe\n", encoding="utf-8")
        run_git("add", "safe.txt")
        run_git("commit", "--quiet", "-m", "safe publishing change")
        safe_checked = run_git("rev-parse", "HEAD")

        deleted_result = publishability_preflight(
            checked_with_deleted_blob, base, repo=repo
        )
        if deleted_result["result"] != "FAIL" or not any(
            item["size"] > HOSTING_HARD_LIMIT_BYTES
            for item in deleted_result["oversized_blobs"]
        ):
            raise AssertionError(
                "deleted oversized blob was not rejected by publishability preflight"
            )
        safe_result = publishability_preflight(safe_checked, base, repo=repo)
        if safe_result["result"] != "PASS":
            raise AssertionError(
                f"unrelated refs changed safe publishability result: {safe_result}"
            )

        run_git("checkout", "--quiet", "-b", "legacy", base)
        legacy_path = repo / MIGRATION_SOURCE_CLASSIFICATIONS_PATH
        legacy_path.parent.mkdir(parents=True, exist_ok=True)
        legacy_path.write_text("legacy monolith\n", encoding="utf-8")
        run_git("add", MIGRATION_SOURCE_CLASSIFICATIONS_PATH)
        run_git("commit", "--quiet", "-m", "introduce forbidden legacy monolith")
        legacy_checked = run_git("rev-parse", "HEAD")
        legacy_result = publishability_preflight(legacy_checked, base, repo=repo)
        if legacy_result["result"] != "FAIL" or not legacy_result["forbidden_legacy_paths"]:
            raise AssertionError("reachable V2 monolith was not rejected")


def migration_parity_regression_cases() -> dict[str, Callable[[], None]]:
    """Return focused one-time V2-to-V3 parity mutations."""
    source_raw, source_records = migration_source_classifications()
    target_records = candidate_ordered_records(source_records)
    upstream_name = f"sources/m2_5/closures/C/{MODEL_NAME}"

    def upstream_mismatch() -> None:
        validate_migration_upstream_parity(upstream_name, b"source", b"tampered")

    def record_mismatch() -> None:
        target = copy.deepcopy(target_records)
        target[0]["review_rationale"] += " tampered"
        migration_parity_evidence(source_raw, source_records, target, {})

    def missing_record() -> None:
        migration_parity_evidence(source_raw, source_records, target_records[:-1], {})

    def duplicate_record() -> None:
        target = copy.deepcopy(target_records)
        target[-1] = copy.deepcopy(target[0])
        migration_parity_evidence(source_raw, source_records, target, {})

    return {
        "migration_upstream_mismatch": upstream_mismatch,
        "migration_record_mismatch": record_mismatch,
        "migration_missing_record": missing_record,
        "migration_duplicate_record": duplicate_record,
    }


def negative_self_test() -> int:
    validator = globals().get("validate_v2_candidate_state")
    if not callable(validator):
        print("FAIL: V3 state grammar validator is not implemented")
        return 1
    state_validator = cast(Callable[[dict[str, Any], str], None], validator)
    valid_unresolved = {
        "review_state": "unresolved",
        "terminal_disposition": None,
        "interaction_class_id": None,
        "unresolved_reason": "insufficient_pair_relation_authority",
    }
    invalid_states = (
        {
            **valid_unresolved,
            "review_state": "resolved",
        },
        {
            **valid_unresolved,
            "terminal_disposition": "required_interaction",
        },
        {
            **valid_unresolved,
            "unresolved_reason": "unknown_reason",
        },
    )
    try:
        state_validator(valid_unresolved, "V2 state regression valid unresolved")
        for index, invalid in enumerate(invalid_states):
            try:
                state_validator(invalid, f"V2 state regression invalid {index}")
            except CCheckError:
                continue
            raise AssertionError(f"invalid V2 state {index} unexpectedly passed")
    except CCheckError as exc:
        print(f"FAIL: V3 state regression unexpectedly rejected valid state: {exc}")
        return 1
    except AssertionError as exc:
        print(f"FAIL: {exc}")
        return 1
    print("V3_STATE_GRAMMAR_REGRESSION = PASS")
    snapshot = load_snapshot()
    matrix = get_artifact(snapshot, MATRIX_NAME)
    validate_matrix(matrix)
    cases = mutation_cases(snapshot)
    failures: list[str] = []
    for item in matrix["cases"]:
        case_id = item["case_id"]
        try:
            cases[case_id]()
        except CCheckError as exc:
            if exc.code != item["expected_reason_code"] or exc.status != item["expected_status"]:
                failures.append(
                    f"{case_id}: expected {item['expected_status']} "
                    f"{item['expected_reason_code']}, got {exc.status} {exc.code}"
                )
            else:
                print(f"NEGATIVE {case_id}: rejected ({exc.status}) [{exc.code}]")
        except Exception as exc:
            failures.append(f"{case_id}: unexpected exception {type(exc).__name__}: {exc}")
        else:
            failures.append(f"{case_id}: mutation unexpectedly passed")
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return 1
    supplemental_failures: list[str] = []
    supplemental = classification_layout_regression_cases(snapshot)
    expected_supplemental = {
        "missing_shard": "CLASSIFICATION_SHARD_MISSING",
        "extra_shard": "UNLISTED_C_ARTIFACT",
        "missing_root": "UNLISTED_C_ARTIFACT",
        "extra_root_entry": "UNLISTED_C_ARTIFACT",
        "root_count_mismatch": "CLASSIFICATION_ROOT_COUNT_MISMATCH",
        "root_records_forbidden": "CLASSIFICATION_LAYOUT_INVALID",
        "shard_root_digest_forbidden": "CLASSIFICATION_LAYOUT_INVALID",
        "wrong_shard_index": "CLASSIFICATION_SHARD_RANGE_MISMATCH",
        "wrong_shard_path": "CLASSIFICATION_SHARD_RANGE_MISMATCH",
        "wrong_shard_digest": "CLASSIFICATION_SHARD_DIGEST_MISMATCH",
        "wrong_record_count": "CLASSIFICATION_SHARD_RANGE_MISMATCH",
        "candidate_in_wrong_shard": "CLASSIFICATION_SHARD_COVERAGE_MISMATCH",
        "duplicate_candidate_across_shards": "CLASSIFICATION_SHARD_COVERAGE_MISMATCH",
        "missing_candidate_across_shards": "CLASSIFICATION_SHARD_COVERAGE_MISMATCH",
        "non_contiguous_reordered_partition": "CLASSIFICATION_LAYOUT_INVALID",
        "shard_over_50_mib": "CLASSIFICATION_SHARD_SIZE_LIMIT",
    }
    for name, test in supplemental.items():
        try:
            test()
        except CCheckError as exc:
            expected_code = expected_supplemental[name]
            if exc.code != expected_code or exc.status != "FAIL":
                supplemental_failures.append(
                    f"{name}: expected FAIL {expected_code}, got {exc.status} {exc.code}"
                )
            else:
                print(f"SUPPLEMENTAL {name}: rejected (FAIL) [{exc.code}]")
        except Exception as exc:
            supplemental_failures.append(
                f"{name}: unexpected exception {type(exc).__name__}: {exc}"
            )
        else:
            supplemental_failures.append(f"{name}: mutation unexpectedly passed")
    if supplemental_failures:
        for failure in supplemental_failures:
            print(f"FAIL: {failure}")
        return 1
    try:
        publishability_history_regression()
    except Exception as exc:
        print(f"FAIL: publishability history regression: {type(exc).__name__}: {exc}")
        return 1
    print("SUPPLEMENTAL publishability_history: rejected deleted oversize; ignored unrelated ref")
    migration_failures: list[str] = []
    migration_cases = migration_parity_regression_cases()
    expected_migration = {
        "migration_upstream_mismatch": "V2_V3_UPSTREAM_PARITY_MISMATCH",
        "migration_record_mismatch": "V2_V3_CLASSIFICATION_PARITY_MISMATCH",
        "migration_missing_record": "V2_V3_CLASSIFICATION_PARITY_MISMATCH",
        "migration_duplicate_record": "V2_V3_CLASSIFICATION_PARITY_MISMATCH",
    }
    for name, test in migration_cases.items():
        try:
            test()
        except CCheckError as exc:
            expected_code = expected_migration[name]
            if exc.code != expected_code or exc.status != "FAIL":
                migration_failures.append(
                    f"{name}: expected FAIL {expected_code}, got {exc.status} {exc.code}"
                )
            else:
                print(f"SUPPLEMENTAL {name}: rejected (FAIL) [{exc.code}]")
        except Exception as exc:
            migration_failures.append(
                f"{name}: unexpected exception {type(exc).__name__}: {exc}"
            )
        else:
            migration_failures.append(f"{name}: mutation unexpectedly passed")
    if migration_failures:
        for failure in migration_failures:
            print(f"FAIL: {failure}")
        return 1
    print(
        "NEGATIVE_SELF_TEST = PASS "
        "(42 rejection cases; exact C-001..C-042 matrix; "
        f"{len(supplemental)} supplemental V3 layout cases; "
        f"{len(migration_cases)} migration cases; publishability history case)"
    )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--generate", action="store_true", help="build the deterministic C source artifacts"
    )
    parser.add_argument("--negative-self-test", action="store_true")
    parser.add_argument(
        "--publishability-preflight",
        action="store_true",
        help="scan H_exec or H_evidence Git objects relative to the verified base",
    )
    parser.add_argument("--checked-commit", help="commit to scan for publishability")
    parser.add_argument(
        "--verified-base-commit", default=EXPECTED_PREVIOUS_MASTER
    )
    parser.add_argument(
        "--require-origin-master",
        action="store_true",
        help="require origin/master to resolve to the verified base before scanning",
    )
    parser.add_argument(
        "--finalize-summary",
        action="store_true",
        help="write the post-H_exec summary from external execution evidence",
    )
    parser.add_argument("--execution-commit")
    parser.add_argument("--results-json", type=Path)
    parser.add_argument("--review-export", type=Path)
    args = parser.parse_args()
    try:
        if args.generate:
            generate_artifacts()
            return 0
        if args.negative_self_test:
            return negative_self_test()
        if args.publishability_preflight:
            if not args.checked_commit:
                parser.error("--publishability-preflight requires --checked-commit")
            result = publishability_preflight(
                args.checked_commit,
                args.verified_base_commit,
                require_origin_master_identity=args.require_origin_master,
            )
            print(json.dumps(result, sort_keys=True))
            if result["result"] != "PASS":
                print(f"FAIL: PUBLISHABLE_HISTORY: {result}")
                return 1
            print(f"PUBLISHABILITY_PRECHECK = PASS ({args.checked_commit})")
            return 0
        if args.finalize_summary:
            if not args.execution_commit or args.results_json is None or args.review_export is None:
                parser.error(
                    "--finalize-summary requires --execution-commit, "
                    "--results-json, and --review-export"
                )
            finalize_summary(args.execution_commit, args.results_json, args.review_export)
            return 0
        snapshot = load_snapshot()
        validate_snapshot(snapshot, run_prereqs=True)
        closure = get_artifact(snapshot, CLOSURE_NAME)
        print("MASTER_DRIFT prerequisite = PASS")
        print(
            "DECLARED_INTERACTION_MODEL_CLOSURE = "
            f"{closure['gate_status']['DECLARED_INTERACTION_MODEL_CLOSURE']}"
        )
        c_result = closure["gate_status"]["DECLARED_INTERACTION_MODEL_CLOSURE"]
        print(f"C_RESULT = {c_result}")
        print(f"CANDIDATES = {closure['candidate_reconciliation']['current_total']}")
        print(f"CLASSES = {closure['semantic_class_metrics']['class_count']}")
        print(f"UNRESOLVED = {closure['review_state_metrics']['unresolved']}")
        return 0 if c_result == "PASS" else 2
    except CCheckError as exc:
        print(f"{exc.status}: {exc.code}: {exc.message}")
        return 2 if exc.status == "BLOCKED" else 1
    except (OSError, KeyError, TypeError, ValueError) as exc:
        print(f"BLOCKED: IMPLEMENTATION_ERROR: {exc}")
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
