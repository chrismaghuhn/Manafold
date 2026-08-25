#!/usr/bin/env python3
"""Execute the two M2.H executable gates on an exact clean source head.

Owned gates:

```text
M2_RUST_PYTHON_PLAYER_WIRE_PARITY
M2_RULES_FREE_PYTHON_ADAPTER_PARITY
```

The authoritative mode requires a clean source tree whose commit equals the
expected target SHA when one is supplied.  ``--development`` runs the same
underlying evidence but can never report an authoritative gate result.

Skeleton relationship: this runner models ``scripts/run_m2_g_gates.py`` with
the following DELIBERATE fixes (plan §Q "deliberate deviations"):

1. Per-node log indices: every evidence node derives its own sequential log
   index at execution time, so no two nodes ever share a log-file prefix
   (the G skeleton assigned one index per gate and reused it for every node
   inside that gate).
2. FAIL-dominant aggregation: any FAIL status makes the aggregate FAIL even
   when NOT_RUN or BLOCKED statuses are also present; otherwise the ranking
   is BLOCKED > NOT_RUN > PASS (the G skeleton let a single NOT_RUN mask a
   co-occurring FAIL).
3. Per-node subprocess timeout: every spawned command carries an explicit
   timeout (default 600 seconds); expiry yields a BLOCKED evidence row with
   a reason instead of hanging the whole runner.
4. Structured startup errors: configuration/validation drift discovered by
   ``main()`` raises :class:`GateConfigurationError`, converted into a short
   diagnostic and exit code 2 instead of an interpreter traceback; manifest
   drift at IMPORT time instead aborts the module load itself with a
   traceback and exit code 1.
5. Single-invocation file-level pytest summaries: python evidence runs each
   whole file once with a single ``-v`` (the G skeleton ran one named test
   at a time under double verbosity) and requires exactly one terminal
   summary line whose pass count equals the pinned expectation with zero
   substitute outcomes.

Startup completeness/drift authorities (all fail closed BEFORE any evidence
executes): the exact-set evidence manifest, the mechanically extracted
player-surface closure (Rust ``PlayerEndpoint`` trait signatures, boundary
error variants, Python ``PlayerClient`` protocol annotations,
``AdapterPlayerClient`` public surface), pinned per-contract
SchemaContractDigest identities, the decoder-registry set relation
(``COMMON_NAMED_CONTRACTS`` + the pinned ``PYTHON_MECHANICAL_ONLY``
exception, never bare three-way equality), and transitive variant-closure
pins cross-checked against the generated contract vocabulary and the JSON
Schemas.
"""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tomllib
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "dist" / "m2-h-verification"
OUTPUT_MARKER = ".mtgml-m2-h-gates-output"
PINNED_TOOLCHAIN: dict[str, str | None] = {"channel": None}

NODE_TIMEOUT_SECONDS = 600
ADAPTER_PACKAGE = "m2-semantic-adapter"
ADAPTER_BINARY_RELATIVE = Path("target") / "debug" / ADAPTER_PACKAGE
SCHEMAS_DIR = ROOT / "schemas"

GATE_WIRE_PARITY = "M2_RUST_PYTHON_PLAYER_WIRE_PARITY"
GATE_ADAPTER_PARITY = "M2_RULES_FREE_PYTHON_ADAPTER_PARITY"

RUNTIME_CONTEXT: dict[str, Path | None] = {"adapter_binary": None}


class GateConfigurationError(Exception):
    """Startup/configuration drift discovered before any evidence ran."""


@dataclass(frozen=True)
class EvidenceDefinition:
    kind: str
    name: str
    surface: str
    package: str | None = None
    expected_passed: int | None = None
    requires_adapter_binary: bool = False


def rust(package: str, name: str, surface: str) -> EvidenceDefinition:
    return EvidenceDefinition("rust", name, surface, package)


def rust_package(package: str, name: str, surface: str, expected: int) -> EvidenceDefinition:
    return EvidenceDefinition(
        "rust_package", name, surface, package=package, expected_passed=expected
    )


def python_file(
    relative: str, surface: str, expected: int, *, requires_adapter_binary: bool = False
) -> EvidenceDefinition:
    return EvidenceDefinition(
        "python",
        f"pytest::{relative}",
        surface,
        expected_passed=expected,
        requires_adapter_binary=requires_adapter_binary,
    )


def build(package: str, surface: str) -> EvidenceDefinition:
    return EvidenceDefinition("build", f"build::{package}", surface, package=package)


def check(name: str, surface: str) -> EvidenceDefinition:
    return EvidenceDefinition("check", name, surface)


WIRE_GOLDEN_NODE = "tests::all_golden_wire_fixtures_roundtrip_canonically"
WIRE_NEGATIVE_NODE = "tests::every_shared_negative_fixture_is_rejected_with_the_expected_code"
CONSTRUCTIVE_PREFIX = "constructive_producer_tests::"

CONSTRUCTIVE_NODES: tuple[str, ...] = (
    f"{CONSTRUCTIVE_PREFIX}information_state_envelope_v2_constructs_the_golden_bytes",
    f"{CONSTRUCTIVE_PREFIX}player_decision_request_v2_constructs_the_golden_bytes",
    f"{CONSTRUCTIVE_PREFIX}observation_envelope_v1_constructs_the_golden_bytes",
    f"{CONSTRUCTIVE_PREFIX}observed_event_envelope_v2_object_moved_constructs_the_golden_bytes",
    f"{CONSTRUCTIVE_PREFIX}decision_response_v2_select_one_constructs_the_golden_bytes",
    f"{CONSTRUCTIVE_PREFIX}player_step_v2_constructs_the_golden_bytes",
    f"{CONSTRUCTIVE_PREFIX}episode_status_terminal_concession_constructs_the_golden_bytes",
)

CHECK_DIGESTS = "check::player_schema_contract_digests"
CHECK_REGISTRY = "check::decoder_registry_relation"
CHECK_VARIANTS = "check::transitive_variant_closures"

PYTEST_WIRE_CONTRACTS = "pytest::python/tests/test_wire_contracts.py"
PYTEST_CONSTRUCTIVE = "pytest::python/tests/test_constructive_producers.py"
PYTEST_SCHEMA_PARITY = "pytest::python/tests/test_schema_parity.py"
PYTEST_ADAPTER_UNIT = "pytest::python/tests/test_m2_adapter_unit.py"
PYTEST_CORE_SCENARIOS = "pytest::python/tests/m2_h/test_m2_h_core_scenarios.py"
PYTEST_REJECTION_SCENARIOS = "pytest::python/tests/m2_h/test_m2_h_rejection_scenarios.py"
PYTEST_ISOLATION_SCENARIOS = "pytest::python/tests/m2_h/test_m2_h_isolation_scenarios.py"
PYTEST_GUARDS = "pytest::python/tests/test_m2_h_rules_free_guards.py"

BUILD_ADAPTER = f"build::{ADAPTER_PACKAGE}"
CARGO_PACKAGE_ADAPTER = f"cargo-package::{ADAPTER_PACKAGE}"

# Expected pytest pass counts measured on the H.6 head
# d8d2a940b2f57867f8931b7808e9b8d539a4f7cf (clean tree); re-pin only after a
# conscious, reviewed change to the addressed suite.
EXPECTED_PYTHON_PASSED: dict[str, int] = {
    PYTEST_WIRE_CONTRACTS: 2,
    PYTEST_CONSTRUCTIVE: 16,
    PYTEST_SCHEMA_PARITY: 7,
    PYTEST_ADAPTER_UNIT: 44,
    PYTEST_CORE_SCENARIOS: 4,
    PYTEST_REJECTION_SCENARIOS: 14,
    PYTEST_ISOLATION_SCENARIOS: 6,
    PYTEST_GUARDS: 5,
}

# Expected whole-package cargo pass count measured on the H.6 head (lib +
# bin + doc targets summed across every "test result:" line); re-pinned 22
# for the token-collision fail-closed seam test added to tokens.rs.
EXPECTED_ADAPTER_PACKAGE_PASSED = 22


GATE_TESTS: dict[str, tuple[EvidenceDefinition, ...]] = {
    GATE_WIRE_PARITY: (
        rust(
            "mtgml-wire",
            WIRE_GOLDEN_NODE,
            "shared golden corpus decodes and re-encodes byte-exactly (Rust)",
        ),
        rust(
            "mtgml-wire",
            WIRE_NEGATIVE_NODE,
            "shared negative corpus rejected with the identical expected codes (Rust)",
        ),
        *(
            rust(
                "mtgml-wire",
                node,
                "constructive producer compiles every DTO field explicitly against "
                f"the checked-in golden bytes ({node.split(CONSTRUCTIVE_PREFIX)[1]})",
            )
            for node in CONSTRUCTIVE_NODES
        ),
        python_file(
            "python/tests/test_wire_contracts.py",
            "python shared-fixture consumer loops: golden round-trips and negatives",
            EXPECTED_PYTHON_PASSED[PYTEST_WIRE_CONTRACTS],
        ),
        python_file(
            "python/tests/test_constructive_producers.py",
            "python constructive producers encode domain data to the shared goldens",
            EXPECTED_PYTHON_PASSED[PYTEST_CONSTRUCTIVE],
        ),
        python_file(
            "python/tests/test_schema_parity.py",
            "python codec/schema parity matrix",
            EXPECTED_PYTHON_PASSED[PYTEST_SCHEMA_PARITY],
        ),
        check(
            CHECK_DIGESTS,
            "SUPPLEMENTAL: SchemaContractDigest identities over the seven "
            "top-level player schemas (field-drift authority)",
        ),
        check(
            CHECK_REGISTRY,
            "SUPPLEMENTAL: decoder registry relation rust == COMMON, "
            "python == COMMON union PYTHON_MECHANICAL_ONLY, schemas == COMMON",
        ),
        check(
            CHECK_VARIANTS,
            "SUPPLEMENTAL: transitive variant closures (event kinds, episode "
            "reasons, answer families, submission codes) across schema, "
            "generated vocabulary, and codecs",
        ),
    ),
    GATE_ADAPTER_PARITY: (
        build(
            ADAPTER_PACKAGE,
            "build the m2-semantic-adapter tool binary (prerequisite for the "
            "adapter-backed nodes below)",
        ),
        rust_package(
            ADAPTER_PACKAGE,
            CARGO_PACKAGE_ADAPTER,
            "m2-semantic-adapter full package suite (framing, tokens, handlers, "
            "transparency proofs, counters)",
            EXPECTED_ADAPTER_PACKAGE_PASSED,
        ),
        python_file(
            "python/tests/test_m2_adapter_unit.py",
            "python adapter transport/protocol/submission/client unit suite",
            EXPECTED_PYTHON_PASSED[PYTEST_ADAPTER_UNIT],
        ),
        python_file(
            "python/tests/m2_h/test_m2_h_core_scenarios.py",
            "lockstep twin core scenarios (explicit chain, accepted parity)",
            EXPECTED_PYTHON_PASSED[PYTEST_CORE_SCENARIOS],
            requires_adapter_binary=True,
        ),
        python_file(
            "python/tests/m2_h/test_m2_h_rejection_scenarios.py",
            "typed reachable rejection classes and malformed raw-byte boundary",
            EXPECTED_PYTHON_PASSED[PYTEST_REJECTION_SCENARIOS],
            requires_adapter_binary=True,
        ),
        python_file(
            "python/tests/m2_h/test_m2_h_isolation_scenarios.py",
            "isolation, paired seeds, and restart determinism scenarios",
            EXPECTED_PYTHON_PASSED[PYTEST_ISOLATION_SCENARIOS],
            requires_adapter_binary=True,
        ),
        python_file(
            "python/tests/test_m2_h_rules_free_guards.py",
            "rules-free static guard inventory",
            EXPECTED_PYTHON_PASSED[PYTEST_GUARDS],
        ),
    ),
}

EXPECTED_EVIDENCE: dict[str, tuple[str, ...]] = {
    GATE_WIRE_PARITY: (
        WIRE_GOLDEN_NODE,
        WIRE_NEGATIVE_NODE,
        *CONSTRUCTIVE_NODES,
        PYTEST_WIRE_CONTRACTS,
        PYTEST_CONSTRUCTIVE,
        PYTEST_SCHEMA_PARITY,
        CHECK_DIGESTS,
        CHECK_REGISTRY,
        CHECK_VARIANTS,
    ),
    GATE_ADAPTER_PARITY: (
        BUILD_ADAPTER,
        CARGO_PACKAGE_ADAPTER,
        PYTEST_ADAPTER_UNIT,
        PYTEST_CORE_SCENARIOS,
        PYTEST_REJECTION_SCENARIOS,
        PYTEST_ISOLATION_SCENARIOS,
        PYTEST_GUARDS,
    ),
}


# ---------------------------------------------------------------------------
# Pinned player-surface closure (completeness authority).
#
# Provenance: extracted and reviewed at M2.H H.7-i against the sources listed
# below on a clean tree.  ANY signature/variant drift anywhere in these
# sources fails startup until this manifest is consciously re-pinned.
# ---------------------------------------------------------------------------

ENDPOINT_RS = ROOT / "crates" / "mtgml-environment" / "src" / "endpoint.rs"
BOUNDARY_RS = ROOT / "crates" / "mtgml-environment" / "src" / "boundary.rs"
WIRE_LIB_RS = ROOT / "crates" / "mtgml-wire" / "src" / "lib.rs"
PLAYER_CLIENT_PY = ROOT / "python" / "src" / "mtgml" / "player_client.py"
ADAPTER_CLIENT_PY = ROOT / "python" / "src" / "mtgml" / "_m2_adapter" / "client.py"
WIRE_PY = ROOT / "python" / "src" / "mtgml" / "wire.py"
VALIDATE_SCHEMAS_PY = ROOT / "scripts" / "validate_schemas.py"

RUST_PLAYER_ENDPOINT_METHODS: dict[str, dict[str, object]] = {
    "perspective": {"params": {}, "returns": "PlayerId"},
    "observation": {
        "params": {},
        "returns": "Result<ObservationEnvelope, PlayerEndpointError>",
    },
    "information_state": {
        "params": {},
        "returns": "Result<PlayerInformationStateV2, PlayerEndpointError>",
    },
    "visible_decision": {
        "params": {},
        "returns": "Result<Option<PlayerDecisionRequestV2>, PlayerEndpointError>",
    },
    "submit": {
        "params": {"response": "DecisionResponseV2"},
        "returns": "Result<PlayerStepV2, PlayerEndpointError>",
    },
}

RUST_PLAYER_BOUNDARY_VARIANTS = frozenset(
    {
        "Wire(PlayerWireErrorCodeV1)",
        "Service(PlayerServiceErrorCodeV1)",
    }
)

PYTHON_PROTOCOL_METHODS: dict[str, dict[str, object]] = {
    "observation": {"params": {}, "returns": "ObservationEnvelope"},
    "information_state": {"params": {}, "returns": "PlayerInformationStateV2"},
    "visible_decision": {"params": {}, "returns": "PlayerDecisionRequestV2 | None"},
    "submit": {
        "params": {"response": "DecisionResponseV2"},
        "returns": "PlayerStepV2",
    },
}

ADAPTER_PUBLIC_METHODS = frozenset(
    {"observation", "information_state", "visible_decision", "submit"}
)


# ---------------------------------------------------------------------------
# Pinned decoder-registry relation (drift regression, NOT completeness).
#
# Plan §G.6: three-way equality would permanently contradict the deliberately
# kept Python-only digest-input registry entry, so the relation below IS the
# accepted contract.
# ---------------------------------------------------------------------------

COMMON_NAMED_CONTRACTS = frozenset(
    {
        "player-decision-request.v1",
        "decision-response.v1",
        "player-decision-request.v2",
        "decision-response.v2",
        "observation-envelope.v1",
        "information-state-envelope.v1",
        "observed-event-envelope.v1",
        "player-step.v1",
        "information-state-envelope.v2",
        "observed-event-envelope.v2",
        "player-step.v2",
        "episode-status.v1",
        "replay-manifest.v1",
        "authoritative-replay.v1",
        "replay-manifest.v2",
        "authoritative-replay.v2",
        "replay-manifest.v3",
        "authoritative-replay.v3",
    }
)

PYTHON_MECHANICAL_ONLY = frozenset({"information-state-digest-input.v2"})


# ---------------------------------------------------------------------------
# Pinned transitive variant closures.
#
# Cross-checked at startup against (a) the checked-in JSON Schemas and (b) the
# generated contract vocabulary / codecs imported from python/src.
# ---------------------------------------------------------------------------

PINNED_OBSERVED_EVENT_KINDS = frozenset(
    {
        "object_moved",
        "object_ceased_to_exist",
        "life_changed",
        "object_tapped",
        "decision_available",
        "random_outcome_visible",
        "public_outcome",
    }
)

PINNED_TERMINAL_REASONS = frozenset(
    {
        "rules_loss",
        "concession",
        "simultaneous_outcome",
        "rules_draw",
        "specified_loop",
    }
)

PINNED_TRUNCATION_REASONS = frozenset(
    {
        "decision_limit",
        "rule_event_limit",
        "wall_clock_limit",
        "resource_limit",
        "external_stop",
    }
)

PINNED_ANSWER_FAMILIES = frozenset({"select_one", "select_many", "order", "choose_number"})

PINNED_SUBMISSION_CODES = frozenset(
    {
        "stale_decision",
        "unavailable_decision",
        "invalid_answer",
        "invalid_candidate",
        "duplicate_assignment",
        "invalid_cardinality",
        "invalid_number",
        "invalid_order",
        "episode_closed",
    }
)


TOP_LEVEL_PLAYER_SCHEMAS: tuple[str, ...] = (
    "observation-envelope.v1.schema.json",
    "information-state-envelope.v2.schema.json",
    "player-decision-request.v2.schema.json",
    "decision-response.v2.schema.json",
    "player-step.v2.schema.json",
    "episode-status.v1.schema.json",
    "observed-event-envelope.v2.schema.json",
)


# ---------------------------------------------------------------------------
# Pinned SchemaContractDigest identities (field-drift authority).
#
# Provenance: computed by THIS module's normalizer (--print-schema-digests)
# over the seven top-level player schemas at M2.H H.7-i against head
# d8d2a940b2f57867f8931b7808e9b8d539a4f7cf (clean tree); re-pin ONLY after
# conscious review of the schema change together with its positive+negative
# coverage.  Digest = sha256 over canonical JSON of the normalized schema
# closure ($refs resolved transitively within schemas/, non-semantic
# annotation keywords dropped, keys sorted recursively).
# ---------------------------------------------------------------------------

SCHEMA_CONTRACT_DIGESTS: dict[str, str] = {
    "observation-envelope.v1": "b3494a12ab4cb036e0847ea9b1f37e26f2ec3b55d93c29c6ddf6dbf0acc580fc",
    "information-state-envelope.v2": (
        "5c4e009ee3e74041e51f48b1bbad66124acc73116fe4fbdbafbd71d41136bd20"
    ),
    "player-decision-request.v2": (
        "297dcf2fbb2dc2d5f16ba7d3c0c8bf0f573013f6c9b67834f8df00eade997568"
    ),
    "decision-response.v2": "364238d3ba62828eb7a56758ffbb5a99456b2858cd75a0b42b721dae5b5feb24",
    "player-step.v2": "1c92a74cf34d19c3588fe7a4e18f78c1867ea6832290d8a4c2194ce0ea8b26fa",
    "episode-status.v1": "b0cf9fc6ecc32d9615a2d3032f386a7e23580678ee40dbcd41195e3855fb9a7a",
    "observed-event-envelope.v2": (
        "d459d6802f2e3450d3501e0912d7a59a8c442c5455428d7e2ad38076c1d2700e"
    ),
}


def _load_schema(name: str) -> dict[str, Any]:
    path = SCHEMAS_DIR / name
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise GateConfigurationError(f"schema unreadable: {name}: {error}") from error
    if not isinstance(value, dict):
        raise GateConfigurationError(f"schema is not an object: {name}")
    return value


_NON_SEMANTIC_KEYWORDS = frozenset(
    {"title", "description", "$comment", "examples", "default", "$schema"}
)


def _resolve_pointer(document: dict[str, Any], pointer: str, context: str) -> Any:
    node: Any = document
    if pointer:
        for raw_token in pointer.lstrip("/").split("/"):
            token = raw_token.replace("~1", "/").replace("~0", "~")
            if isinstance(node, dict) and token in node:
                node = node[token]
            elif isinstance(node, list) and token.lstrip("-").isdigit():
                index = int(token)
                if 0 <= index < len(node):
                    node = node[index]
                else:
                    raise GateConfigurationError(
                        f"unresolvable $ref pointer {pointer!r} ({context})"
                    )
            else:
                raise GateConfigurationError(f"unresolvable $ref pointer {pointer!r} ({context})")
    return node


def _normalize_schema_node(
    node: Any, document: dict[str, Any], source_name: str, seen: tuple[str, ...]
) -> Any:
    if isinstance(node, list):
        return [_normalize_schema_node(item, document, source_name, seen) for item in node]
    if not isinstance(node, dict):
        return node
    ref = node.get("$ref")
    if isinstance(ref, str):
        if (source_name, ref) in seen:
            raise GateConfigurationError(
                f"cyclic $ref closure in {source_name}: {' -> '.join((*seen, ref))}"
            )
        if ref.startswith("#"):
            target_document, pointer = document, ref[1:]
            target_source = source_name
        else:
            file_name, separator, pointer = ref.partition("#")
            if not file_name.endswith(".json"):
                raise GateConfigurationError(f"unsupported $ref target {ref!r} in {source_name}")
            target_document = _load_schema(file_name)
            pointer = pointer if separator else ""
            target_source = file_name
        resolved = _resolve_pointer(target_document, pointer, f"{source_name}: {ref}")
        normalized_target = _normalize_schema_node(
            resolved, target_document, target_source, (*seen, (source_name, ref))
        )
        remainder = {key: value for key, value in node.items() if key != "$ref"}
        if not remainder:
            return normalized_target
        if not isinstance(normalized_target, dict):
            raise GateConfigurationError(
                f"$ref with siblings resolved to a non-object in {source_name}: {ref!r}"
            )
        merged: dict[str, Any] = dict(normalized_target)
        for key, value in _normalize_schema_node(remainder, document, source_name, seen).items():
            merged[key] = value
        return merged
    normalized: dict[str, Any] = {}
    for key in sorted(node):
        if key in _NON_SEMANTIC_KEYWORDS:
            continue
        normalized[key] = _normalize_schema_node(node[key], document, source_name, seen)
    return normalized


def schema_contract_digest(schema_name: str) -> str:
    """sha256 over canonical JSON of the normalized validation-relevant closure."""
    document = _load_schema(schema_name)
    normalized = _normalize_schema_node(document, document, schema_name, ())
    serialized = json.dumps(normalized, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(serialized.encode("utf-8")).hexdigest()


# ---------------------------------------------------------------------------
# Mechanical source extractors.
# ---------------------------------------------------------------------------


def _norm_type(text: str) -> str:
    return re.sub(r"\s+", "", text)


def _rust_balanced_block(text: str, start_marker: str, origin: str) -> str:
    position = text.find(start_marker)
    if position < 0:
        raise GateConfigurationError(f"missing {start_marker!r} in {origin}")
    open_brace = text.find("{", position)
    if open_brace < 0:
        raise GateConfigurationError(f"unbraced {start_marker!r} in {origin}")
    depth = 0
    for index in range(open_brace, len(text)):
        character = text[index]
        if character == "{":
            depth += 1
        elif character == "}":
            depth -= 1
            if depth == 0:
                return text[position : index + 1]
    raise GateConfigurationError(f"unbalanced braces for {start_marker!r} in {origin}")


def _read_source(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise GateConfigurationError(f"unreadable source {path}: {error}") from error


def extract_rust_trait_methods(origin: str) -> dict[str, dict[str, object]]:
    block = _rust_balanced_block(_read_source(ENDPOINT_RS), "pub trait PlayerEndpoint", origin)
    methods: dict[str, dict[str, object]] = {}
    pattern = re.compile(r"\n\s*(?:pub\s+)?fn\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*([^;{]+))?")
    for match in pattern.finditer(block):
        name = match.group(1)
        raw_params = match.group(2) or ""
        returns = _norm_type(match.group(3) or "")
        params: dict[str, str] = {}
        for piece in raw_params.split(","):
            piece = piece.strip()
            if not piece or re.fullmatch(r"(?:&\s*)?(?:mut\s+)?self", piece):
                continue
            param_name, separator, param_type = piece.partition(":")
            if not separator:
                raise GateConfigurationError(f"{origin}: untyped parameter {piece!r} on {name}")
            params[param_name.strip()] = _norm_type(param_type)
        methods[name] = {"params": params, "returns": returns}
    if not methods:
        raise GateConfigurationError(f"{origin}: no trait methods extracted")
    return methods


def extract_rust_enum_variants(origin: str) -> frozenset[str]:
    block = _rust_balanced_block(_read_source(BOUNDARY_RS), "pub enum PlayerBoundaryError", origin)
    variants = re.findall(r"\n\s{4}([A-Z]\w*)\(([^)]*)\)", block)
    if not variants:
        raise GateConfigurationError(f"{origin}: no enum variants extracted")
    return frozenset(f"{name}({_norm_type(payload)})" for name, payload in variants)


def _python_class(path: Path, class_name: str, origin: str) -> ast.ClassDef:
    try:
        tree = ast.parse(_read_source(path), filename=str(path))
    except SyntaxError as error:
        raise GateConfigurationError(f"{origin}: unparsable python source: {error}") from error
    for node in tree.body:
        if isinstance(node, ast.ClassDef) and node.name == class_name:
            return node
    raise GateConfigurationError(f"{origin}: class {class_name} not found")


def extract_python_protocol_methods(origin: str) -> dict[str, dict[str, object]]:
    klass = _python_class(PLAYER_CLIENT_PY, "PlayerClient", origin)
    bases = {base.id for base in klass.bases if isinstance(base, ast.Name)}
    if "Protocol" not in bases:
        raise GateConfigurationError(f"{origin}: PlayerClient is not a Protocol")
    methods: dict[str, dict[str, object]] = {}
    for item in klass.body:
        if not isinstance(item, ast.FunctionDef | ast.AsyncFunctionDef):
            continue
        params: dict[str, str] = {}
        for argument in item.args.args:
            if argument.arg in ("self", "cls"):
                continue
            if argument.annotation is None:
                raise GateConfigurationError(
                    f"{origin}: unannotated parameter {argument.arg} on {item.name}"
                )
            params[argument.arg] = ast.unparse(argument.annotation)
        if item.returns is None:
            raise GateConfigurationError(f"{origin}: unannotated return on {item.name}")
        methods[item.name] = {
            "params": params,
            "returns": ast.unparse(item.returns),
        }
    if not methods:
        raise GateConfigurationError(f"{origin}: no protocol methods extracted")
    return methods


def extract_adapter_public_methods(origin: str) -> frozenset[str]:
    klass = _python_class(ADAPTER_CLIENT_PY, "AdapterPlayerClient", origin)
    names = {
        item.name
        for item in klass.body
        if isinstance(item, ast.FunctionDef | ast.AsyncFunctionDef)
        and not item.name.startswith("_")
    }
    if not names:
        raise GateConfigurationError(f"{origin}: no public methods extracted")
    return frozenset(names)


def extract_rust_decode_named_contracts(origin: str) -> frozenset[str]:
    text = _read_source(WIRE_LIB_RS)
    start = text.find("fn decode_named(")
    if start < 0:
        raise GateConfigurationError(f"{origin}: decode_named not found")
    end_marker = "#[derive(Debug, Error)]"
    end = text.find(end_marker, start)
    if end < 0:
        raise GateConfigurationError(f"{origin}: decode_named end marker not found")
    body = text[start:end]
    names = re.findall(r'"([A-Za-z0-9.\-]+\.v\d+)"\s*=>', body)
    if not names:
        raise GateConfigurationError(f"{origin}: no decode_named arms extracted")
    return frozenset(names)


def _extract_module_dict_keys(path: Path, variable: str, origin: str) -> frozenset[str]:
    try:
        tree = ast.parse(_read_source(path), filename=str(path))
    except SyntaxError as error:
        raise GateConfigurationError(f"{origin}: unparsable python source: {error}") from error
    for node in ast.walk(tree):
        targets: list[ast.expr] = []
        if isinstance(node, ast.Assign):
            targets = node.targets
        elif isinstance(node, ast.AnnAssign):
            targets = [node.target]
        for target in targets:
            if (
                isinstance(target, ast.Name)
                and target.id == variable
                and isinstance(node.value, ast.Dict)
            ):
                keys = [
                    key.value
                    for key in node.value.keys
                    if isinstance(key, ast.Constant) and isinstance(key.value, str)
                ]
                if not keys:
                    continue
                return frozenset(keys)
    raise GateConfigurationError(f"{origin}: {variable} mapping not found")


# Generated vocabulary + codec constants imported from python/src.
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml._generated_contract_vocab import (
    OBSERVED_EVENT_KINDS,
    TerminalReason,
    TruncationReason,
)
from mtgml.observation import PLAYER_SUBMISSION_CODES

# ---------------------------------------------------------------------------
# Startup validators (fail closed BEFORE any evidence executes).
# ---------------------------------------------------------------------------


def validate_gate_manifest() -> None:
    declared_gates = set(GATE_TESTS)
    expected_gates = set(EXPECTED_EVIDENCE)
    if declared_gates != expected_gates:
        raise GateConfigurationError(
            "gate manifest drift: "
            f"unmanifested={sorted(declared_gates - expected_gates)} "
            f"undeclared={sorted(expected_gates - declared_gates)}"
        )
    seen_across_gates: set[str] = set()
    for gate_name, definitions in GATE_TESTS.items():
        names = tuple(definition.name for definition in definitions)
        duplicates = sorted({name for name in names if names.count(name) > 1})
        if duplicates:
            raise GateConfigurationError(
                f"{gate_name}: duplicate evidence node registration {duplicates}"
            )
        expected = EXPECTED_EVIDENCE[gate_name]
        if len(expected) != len(set(expected)):
            raise GateConfigurationError(
                f"{gate_name}: expected manifest itself contains duplicates"
            )
        if list(names) != list(expected):
            held, wanted = set(names), set(expected)
            raise GateConfigurationError(
                f"{gate_name}: evidence manifest drift "
                f"missing={sorted(wanted - held)} extra={sorted(held - wanted)} "
                f"(order must match EXPECTED_EVIDENCE)"
            )
        shared = seen_across_gates.intersection(names)
        if shared:
            raise GateConfigurationError(
                f"duplicate evidence node registered across gates: {sorted(shared)}"
            )
        seen_across_gates.update(names)


def verify_player_surface_closure() -> str:
    origin = "player-surface closure"
    extracted_rust = extract_rust_trait_methods(origin)
    expected_rust = {
        name: {
            "params": {
                param: _norm_type(value)  # type: ignore[union-attr]
                for param, value in signature["params"].items()  # type: ignore[union-attr]
            },
            "returns": _norm_type(str(signature["returns"])),
        }
        for name, signature in RUST_PLAYER_ENDPOINT_METHODS.items()
    }
    if extracted_rust != expected_rust:
        raise GateConfigurationError(
            f"{origin}: PlayerEndpoint trait drift: "
            f"extracted={extracted_rust} pinned={expected_rust}"
        )
    extracted_boundary = extract_rust_enum_variants(origin)
    expected_boundary = frozenset(map(_norm_type, RUST_PLAYER_BOUNDARY_VARIANTS))
    if extracted_boundary != expected_boundary:
        raise GateConfigurationError(
            f"{origin}: PlayerBoundaryError variant drift: "
            f"extracted={sorted(extracted_boundary)} pinned={sorted(expected_boundary)}"
        )
    extracted_protocol = extract_python_protocol_methods(origin)
    expected_protocol = {
        name: {
            "params": dict(signature["params"]),  # type: ignore[arg-type]
            "returns": str(signature["returns"]),
        }
        for name, signature in PYTHON_PROTOCOL_METHODS.items()
    }
    if extracted_protocol != expected_protocol:
        raise GateConfigurationError(
            f"{origin}: PlayerClient protocol drift: "
            f"extracted={extracted_protocol} pinned={expected_protocol}"
        )
    extracted_adapter = extract_adapter_public_methods(origin)
    if extracted_adapter != ADAPTER_PUBLIC_METHODS:
        raise GateConfigurationError(
            f"{origin}: AdapterPlayerClient public surface drift: "
            f"extracted={sorted(extracted_adapter)} "
            f"pinned={sorted(ADAPTER_PUBLIC_METHODS)}"
        )
    return (
        f"closure holds: {len(expected_rust)} rust trait methods, "
        f"{len(expected_boundary)} boundary variants, "
        f"{len(expected_protocol)} python protocol methods, "
        f"{len(extracted_adapter)} adapter public methods"
    )


def verify_schema_contract_digests() -> str:
    drifted = {}
    for schema_name in TOP_LEVEL_PLAYER_SCHEMAS:
        current = schema_contract_digest(schema_name)
        contract = schema_name.removesuffix(".schema.json")
        if current != SCHEMA_CONTRACT_DIGESTS.get(contract):
            drifted[contract] = current
    if drifted:
        raise GateConfigurationError(
            "SchemaContractDigest drift (update pinned manifest coherently with "
            f"positive+negative coverage): {drifted}"
        )
    return f"{len(TOP_LEVEL_PLAYER_SCHEMAS)} pinned SchemaContractDigests match"


def verify_registry_relation() -> str:
    origin = "decoder registry relation"
    rust_arms = extract_rust_decode_named_contracts(origin)
    python_decoders = _extract_module_dict_keys(WIRE_PY, "_DECODERS", origin)
    schema_mapping = _extract_module_dict_keys(VALIDATE_SCHEMAS_PY, "WIRE_MAPPING", origin)
    problems: list[str] = []
    if rust_arms != COMMON_NAMED_CONTRACTS:
        problems.append(
            f"rust decode_named != COMMON: "
            f"extra={sorted(rust_arms - COMMON_NAMED_CONTRACTS)} "
            f"missing={sorted(COMMON_NAMED_CONTRACTS - rust_arms)}"
        )
    expected_python = COMMON_NAMED_CONTRACTS | PYTHON_MECHANICAL_ONLY
    if python_decoders != expected_python:
        problems.append(
            f"python _DECODERS != COMMON union PYTHON_MECHANICAL_ONLY: "
            f"extra={sorted(python_decoders - expected_python)} "
            f"missing={sorted(expected_python - python_decoders)}"
        )
    if schema_mapping != COMMON_NAMED_CONTRACTS:
        problems.append(
            f"schemas WIRE_MAPPING != COMMON: "
            f"extra={sorted(schema_mapping - COMMON_NAMED_CONTRACTS)} "
            f"missing={sorted(COMMON_NAMED_CONTRACTS - schema_mapping)}"
        )
    if problems:
        raise GateConfigurationError(f"{origin}: " + "; ".join(problems))
    return (
        f"relation holds: |COMMON|={len(COMMON_NAMED_CONTRACTS)}, "
        f"|python exception|={len(PYTHON_MECHANICAL_ONLY)}"
    )


def _schema_alternative_kinds(
    alternatives: Any, source_name: str, label: str
) -> list[tuple[Any, dict[str, Any]]]:
    rows: list[tuple[Any, dict[str, Any]]] = []
    if not isinstance(alternatives, list):
        raise GateConfigurationError(f"{source_name}: {label} oneOf is not a list")
    for alternative in alternatives:
        if not isinstance(alternative, dict):
            raise GateConfigurationError(f"{source_name}: {label} alternative malformed")
        properties = alternative.get("properties")
        kind_value: Any = None
        if isinstance(properties, dict):
            kind_schema = properties.get("kind")
            if isinstance(kind_schema, dict):
                kind_value = kind_schema.get("const")
        rows.append((kind_value, alternative))
    return rows


def _schema_kind_consts(alternatives: Any, source_name: str, label: str) -> frozenset[str]:
    rows = _schema_alternative_kinds(alternatives, source_name, label)
    kinds: set[str] = set()
    for kind, _ in rows:
        if not isinstance(kind, str):
            raise GateConfigurationError(f"{source_name}: {label} alternative lacks a const kind")
        kinds.add(kind)
    if len(kinds) != len(rows):
        raise GateConfigurationError(
            f"{source_name}: {label} alternatives carry duplicate const kinds"
        )
    return frozenset(kinds)


def verify_variant_closures() -> str:
    origin = "transitive variant closure"
    problems: list[str] = []

    events = _load_schema("observed-event-envelope.v2.schema.json")
    event_defs = events.get("$defs")
    if not isinstance(event_defs, dict) or "event" not in event_defs:
        raise GateConfigurationError(f"{origin}: observed-event v2 lacks $defs.event")
    event_schema = event_defs["event"]
    event_one_of = event_schema.get("oneOf") if isinstance(event_schema, dict) else None
    schema_event_kinds = _schema_kind_consts(event_one_of, "observed-event-envelope.v2", "event")
    if schema_event_kinds != PINNED_OBSERVED_EVENT_KINDS:
        problems.append(
            f"event kinds != pinned: schema-extra="
            f"{sorted(schema_event_kinds - PINNED_OBSERVED_EVENT_KINDS)} "
            f"pinned-only={sorted(PINNED_OBSERVED_EVENT_KINDS - schema_event_kinds)}"
        )
    if set(OBSERVED_EVENT_KINDS) != PINNED_OBSERVED_EVENT_KINDS:
        problems.append(
            f"event kinds != generated vocabulary: "
            f"generated-extra={sorted(set(OBSERVED_EVENT_KINDS) - PINNED_OBSERVED_EVENT_KINDS)}"
        )

    episode = _load_schema("episode-status.v1.schema.json")
    terminal_reasons: set[str] = set()
    truncation_reasons: set[str] = set()
    for kind, alternative in _schema_alternative_kinds(
        episode.get("oneOf"), "episode-status.v1", "top-level"
    ):
        if kind not in ("terminal", "truncated"):
            continue
        properties = alternative.get("properties") if isinstance(alternative, dict) else None
        reason = properties.get("reason") if isinstance(properties, dict) else None
        values = reason.get("enum") if isinstance(reason, dict) else None
        if not isinstance(values, list):
            raise GateConfigurationError(
                f"{origin}: episode-status.v1 {kind} arm lacks a reason enum"
            )
        (terminal_reasons if kind == "terminal" else truncation_reasons).update(values)
    if terminal_reasons != PINNED_TERMINAL_REASONS:
        problems.append(f"terminal reasons drifted: {sorted(terminal_reasons)}")
    if truncation_reasons != PINNED_TRUNCATION_REASONS:
        problems.append(f"truncation reasons drifted: {sorted(truncation_reasons)}")
    if {member.value for member in TerminalReason} != PINNED_TERMINAL_REASONS:
        problems.append("terminal reasons != generated vocabulary")
    if {member.value for member in TruncationReason} != PINNED_TRUNCATION_REASONS:
        problems.append("truncation reasons != generated vocabulary")

    response_schema = _load_schema("decision-response.v2.schema.json")
    answer_schema = response_schema.get("properties", {}).get("answer")
    answer_one_of = answer_schema.get("oneOf") if isinstance(answer_schema, dict) else None
    schema_families = _schema_kind_consts(answer_one_of, "decision-response.v2", "answer")
    if schema_families != PINNED_ANSWER_FAMILIES:
        problems.append(
            f"answer families != pinned: "
            f"schema-extra={sorted(schema_families - PINNED_ANSWER_FAMILIES)} "
            f"pinned-only={sorted(PINNED_ANSWER_FAMILIES - schema_families)}"
        )

    step = _load_schema("player-step.v2.schema.json")
    step_defs = step.get("$defs")
    submission_code = step_defs.get("submission_code") if isinstance(step_defs, dict) else None
    schema_codes = (
        frozenset(submission_code.get("enum"))
        if isinstance(submission_code, dict) and isinstance(submission_code.get("enum"), list)
        else None
    )
    if schema_codes is None:
        raise GateConfigurationError(f"{origin}: player-step.v2 lacks $defs.submission_code.enum")
    if schema_codes != PINNED_SUBMISSION_CODES:
        problems.append(
            f"submission codes != pinned: "
            f"schema-extra={sorted(schema_codes - PINNED_SUBMISSION_CODES)} "
            f"pinned-only={sorted(PINNED_SUBMISSION_CODES - schema_codes)}"
        )
    if set(PLAYER_SUBMISSION_CODES) != PINNED_SUBMISSION_CODES:
        problems.append("submission codes != mtgml.observation.PLAYER_SUBMISSION_CODES")

    if problems:
        raise GateConfigurationError(f"{origin}: " + "; ".join(problems))
    return (
        "closures hold: "
        f"{len(PINNED_OBSERVED_EVENT_KINDS)} event kinds, "
        f"{len(PINNED_TERMINAL_REASONS)}+{len(PINNED_TRUNCATION_REASONS)} episode reasons, "
        f"{len(PINNED_ANSWER_FAMILIES)} answer families, "
        f"{len(PINNED_SUBMISSION_CODES)} submission codes"
    )


CHECKS = {
    CHECK_DIGESTS: verify_schema_contract_digests,
    CHECK_REGISTRY: verify_registry_relation,
    CHECK_VARIANTS: verify_variant_closures,
}

# Exact-set manifest validation happens at IMPORT time (G pattern): drift
# aborts the module load itself, before main() can even parse arguments.
validate_gate_manifest()


# ---------------------------------------------------------------------------
# Execution machinery.
# ---------------------------------------------------------------------------


def run_command(
    command: Sequence[str], extra_env: dict[str, str] | None = None
) -> subprocess.CompletedProcess[str] | subprocess.TimeoutExpired[str] | OSError:
    environment = dict(os.environ)
    environment["CARGO_TERM_COLOR"] = "never"
    environment["PYTHONDONTWRITEBYTECODE"] = "1"
    if extra_env:
        environment.update(extra_env)
    try:
        return subprocess.run(
            list(command),
            cwd=ROOT,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
            timeout=NODE_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired as error:
        return error
    except OSError as error:
        return error


def command_available(command: Sequence[str]) -> bool:
    return bool(command) and shutil.which(command[0]) is not None


def git_value(arguments: Sequence[str]) -> str:
    completed = run_command(("git", *arguments))
    if isinstance(completed, subprocess.TimeoutExpired | OSError):
        raise RuntimeError(f"git command failed: {completed!r}")
    if completed.returncode != 0:
        raise RuntimeError(completed.stdout.strip() or "git command failed")
    return completed.stdout.strip()


def tracked_source_fingerprint() -> str:
    listed = run_command(("git", "ls-files", "-z"))
    if not isinstance(listed, subprocess.CompletedProcess) or listed.returncode != 0:
        raise RuntimeError("git ls-files failed")
    hasher = hashlib.sha256()
    for encoded in listed.stdout.encode("utf-8").split(b"\0"):
        if not encoded:
            continue
        relative = encoded.decode("utf-8")
        payload = (ROOT / relative).read_bytes()
        hasher.update(len(relative.encode("utf-8")).to_bytes(8, "big"))
        hasher.update(relative.encode("utf-8"))
        hasher.update(len(payload).to_bytes(8, "big"))
        hasher.update(payload)
    return hasher.hexdigest()


def source_snapshot() -> dict[str, Any]:
    try:
        status = git_value(("status", "--porcelain=v1", "--untracked-files=all"))
        return {
            "clean": not status,
            "git_status": status,
            "commit": git_value(("rev-parse", "HEAD")),
            "tree": git_value(("rev-parse", "HEAD^{tree}")),
            "fingerprint": tracked_source_fingerprint(),
        }
    except (OSError, RuntimeError) as error:
        return {"clean": False, "reason": str(error)}


def toolchain_snapshot() -> dict[str, Any]:
    try:
        expected_python = (ROOT / ".python-version").read_text(encoding="utf-8").strip()
        with (ROOT / "rust-toolchain.toml").open("rb") as handle:
            expected_rust = str(tomllib.load(handle)["toolchain"]["channel"])
    except (OSError, KeyError, tomllib.TOMLDecodeError) as error:
        PINNED_TOOLCHAIN["channel"] = None
        return {"status": "BLOCKED", "reason": f"toolchain policy unreadable: {error}"}
    PINNED_TOOLCHAIN["channel"] = expected_rust

    python_version = platform.python_version()
    python_ok = python_version == expected_python
    rust_results: dict[str, Any] = {}
    pinned = f"+{expected_rust}"
    for name, command in (
        ("rustc", ("rustc", pinned, "--version")),
        ("cargo", ("cargo", pinned, "--version")),
    ):
        if not command_available(command):
            rust_results[name] = {"status": "NOT_RUN"}
            continue
        completed = run_command(command)
        output = (
            completed.stdout.strip() if isinstance(completed, subprocess.CompletedProcess) else ""
        )
        match = re.match(rf"^{name}\s+(\d+\.\d+\.\d+)", output.splitlines()[0] if output else "")
        reported = match.group(1) if match else None
        rust_results[name] = {
            "reported": reported,
            "status": "PASS" if reported == expected_rust else "FAIL",
        }
    statuses = [
        "PASS" if python_ok else "FAIL",
        *(item["status"] for item in rust_results.values()),
    ]
    # Statuses here only ever hold PASS, FAIL, or NOT_RUN (unavailable
    # commands are recorded NOT_RUN and fail the snapshot closed), so a
    # BLOCKED outcome cannot arise.
    overall = "PASS" if all(status == "PASS" for status in statuses) else "FAIL"
    return {
        "status": overall,
        "python": {"version": python_version, "expected": expected_python},
        "rust": {"expected": expected_rust, **rust_results},
    }


def prepare_output(output: Path) -> Path:
    relative = output.relative_to(ROOT)
    if "dist" not in relative.parts or output == ROOT:
        raise RuntimeError("M2.H verification output must remain below repository dist")
    if "verification" in relative.parts:
        raise RuntimeError("dist/verification is exclusively owned by release-candidate")
    if output.exists():
        marker = output / OUTPUT_MARKER
        if not marker.is_file():
            raise RuntimeError(f"refusing to replace unowned verification output: {output}")
        shutil.rmtree(output)
    logs = output / "logs"
    logs.mkdir(parents=True, exist_ok=True)
    (output / OUTPUT_MARKER).write_text("owned by scripts/run_m2_h_gates.py\n", encoding="utf-8")
    return logs


def _slug(name: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]+", "-", name).strip("-")


def _captured_timeout_output(error: Any) -> str:
    partial = getattr(error, "stdout", None)
    if isinstance(partial, bytes):
        return partial.decode("utf-8", errors="replace")
    return partial if isinstance(partial, str) else ""


def _apply_timeout_outcome(evidence: dict[str, Any], captured_output: str) -> None:
    evidence["status"] = "BLOCKED"
    evidence["reason"] = f"subprocess exceeded the {NODE_TIMEOUT_SECONDS}s per-node timeout"
    evidence["timeout_partial_output_bytes"] = len(captured_output.encode("utf-8"))


def execute_rust_exact(definition: EvidenceDefinition, log_path: Path) -> dict[str, Any]:
    assert definition.package is not None
    pinned = PINNED_TOOLCHAIN["channel"]
    cargo = ("cargo", f"+{pinned}") if pinned else ("cargo",)
    command = (
        *cargo,
        "test",
        "--package",
        definition.package,
        "--locked",
        "--lib",
        "--",
        definition.name,
        "--exact",
    )
    evidence: dict[str, Any] = {
        "package": definition.package,
        "test": definition.name,
        "command": list(command),
    }
    if not command_available(command):
        evidence.update({"status": "NOT_RUN", "reason": "cargo not found"})
        log_path.write_text("cargo not found\n", encoding="utf-8")
        return evidence
    completed = run_command(command)
    if isinstance(completed, subprocess.TimeoutExpired | OSError):
        output = _captured_timeout_output(completed)
        _apply_timeout_outcome(evidence, output)
    else:
        output = completed.stdout
        passed = bool(
            completed.returncode == 0
            and re.search(r"running\s+1\s+test\b", output)
            and re.search(r"test result:\s+ok\.\s+1 passed;\s+0 failed\b", output)
        )
        evidence.update(
            {
                "status": "PASS" if passed else "FAIL",
                "returncode": completed.returncode,
                "tests_observed": 1 if re.search(r"running\s+1\s+test\b", output) else 0,
                "reason": "exact test passed" if passed else "exact test did not pass",
            }
        )
    log_path.write_text(output, encoding="utf-8")
    return evidence


_CARGO_RESULT_LINE = re.compile(r"test result:\s*(\w+)\.\s+(\d+) passed;\s+(\d+) failed")


def execute_rust_package(definition: EvidenceDefinition, log_path: Path) -> dict[str, Any]:
    assert definition.package is not None and definition.expected_passed is not None
    pinned = PINNED_TOOLCHAIN["channel"]
    cargo = ("cargo", f"+{pinned}") if pinned else ("cargo",)
    command = (*cargo, "test", "--package", definition.package, "--locked")
    evidence: dict[str, Any] = {
        "package": definition.package,
        "test": definition.name,
        "command": list(command),
    }
    if not command_available(command):
        evidence.update({"status": "NOT_RUN", "reason": "cargo not found"})
        log_path.write_text("cargo not found\n", encoding="utf-8")
        return evidence
    completed = run_command(command)
    if isinstance(completed, subprocess.TimeoutExpired | OSError):
        output = _captured_timeout_output(completed)
        _apply_timeout_outcome(evidence, output)
        log_path.write_text(output, encoding="utf-8")
        return evidence
    output = completed.stdout
    log_path.write_text(output, encoding="utf-8")
    total_passed = 0
    total_failed = 0
    any_non_ok = False
    for match in _CARGO_RESULT_LINE.finditer(output):
        verdict, passed, failed = match.group(1), int(match.group(2)), int(match.group(3))
        total_passed += passed
        total_failed += failed
        if verdict != "ok":
            any_non_ok = True
    expected = definition.expected_passed
    passed_check = (
        completed.returncode == 0
        and not any_non_ok
        and total_failed == 0
        and total_passed == expected
    )
    evidence.update(
        {
            "status": "PASS" if passed_check else "FAIL",
            "returncode": completed.returncode,
            "tests_observed": total_passed,
            "expected_passed": expected,
            "reason": (
                f"whole-package summary matched {expected} passed / 0 failed"
                if passed_check
                else f"summary mismatch: {total_passed} passed / {total_failed} failed"
            ),
        }
    )
    return evidence


_PYTHON_OUTCOME = re.compile(
    r"(?<![\w-])(\d+)\s+(passed|failed|error|skipped|xfailed|xpassed|deselected|warnings?)\b"
)


def execute_python_file(definition: EvidenceDefinition, log_path: Path) -> dict[str, Any]:
    assert definition.expected_passed is not None
    relative = definition.name.removeprefix("pytest::")
    # Skeleton deviation (plan §Q): whole-file pytest with a SINGLE -v. The
    # G skeleton needed ``-v -v`` because its exact-name match required the
    # per-node status lines that pytest 9 hides at net-default verbosity
    # (pytest.ini pins global ``addopts = -q ...``); this validator instead
    # demands exactly one terminal summary line whose pass count equals the
    # pin with zero substitute outcomes, which survives net-default output.
    command = (sys.executable, "-m", "pytest", "-v", relative)
    evidence: dict[str, Any] = {
        "package": None,
        "test": definition.name,
        "command": list(command),
    }
    extra_env: dict[str, str] = {}
    if definition.requires_adapter_binary:
        binary = RUNTIME_CONTEXT["adapter_binary"]
        if binary is None:
            evidence.update(
                {
                    "status": "NOT_RUN",
                    "reason": "adapter binary unavailable (build prerequisite failed)",
                }
            )
            log_path.write_text("adapter binary unavailable\n", encoding="utf-8")
            return evidence
        extra_env["MTGML_M2_ADAPTER_BIN"] = str(binary)
    completed = run_command(command, extra_env=extra_env or None)
    if isinstance(completed, subprocess.TimeoutExpired | OSError):
        output = _captured_timeout_output(completed)
        _apply_timeout_outcome(evidence, output)
        log_path.write_text(output, encoding="utf-8")
        return evidence
    output = completed.stdout
    log_path.write_text(output, encoding="utf-8")
    expected = definition.expected_passed
    summaries = [line for line in output.splitlines() if _PYTHON_OUTCOME.search(line)]
    passed = substitutes = warnings = 0
    if len(summaries) == 1:
        for count, kind in _PYTHON_OUTCOME.findall(summaries[0]):
            if kind == "passed":
                passed += int(count)
            elif kind in ("warning", "warnings"):
                warnings += int(count)
            else:
                substitutes += int(count)
    passed_check = (
        completed.returncode == 0
        and len(summaries) == 1
        and passed == expected
        and substitutes == 0
    )
    evidence.update(
        {
            "status": "PASS" if passed_check else "FAIL",
            "returncode": completed.returncode,
            "tests_observed": passed,
            "expected_passed": expected,
            "warnings": warnings,
            "reason": (
                f"summary matched exactly {expected} passed with zero substitute outcomes"
                if passed_check
                else (
                    f"summary mismatch: {passed} passed / {substitutes} substitute "
                    f"outcomes across {len(summaries)} summary lines"
                )
            ),
        }
    )
    return evidence


def execute_build(definition: EvidenceDefinition, log_path: Path) -> dict[str, Any]:
    assert definition.package is not None
    pinned = PINNED_TOOLCHAIN["channel"]
    cargo = ("cargo", f"+{pinned}") if pinned else ("cargo",)
    command = (*cargo, "build", "--package", definition.package, "--locked")
    evidence: dict[str, Any] = {
        "package": definition.package,
        "test": definition.name,
        "command": list(command),
    }
    if not command_available(command):
        evidence.update({"status": "NOT_RUN", "reason": "cargo not found"})
        log_path.write_text("cargo not found\n", encoding="utf-8")
        RUNTIME_CONTEXT["adapter_binary"] = None
        return evidence
    completed = run_command(command)
    if isinstance(completed, subprocess.TimeoutExpired | OSError):
        output = _captured_timeout_output(completed)
        _apply_timeout_outcome(evidence, output)
        log_path.write_text(output, encoding="utf-8")
        RUNTIME_CONTEXT["adapter_binary"] = None
        return evidence
    output = completed.stdout
    log_path.write_text(output, encoding="utf-8")
    suffix = ".exe" if sys.platform == "win32" else ""
    binary = ROOT / ADAPTER_BINARY_RELATIVE.parent / f"{ADAPTER_PACKAGE}{suffix}"
    if completed.returncode == 0 and binary.is_file():
        RUNTIME_CONTEXT["adapter_binary"] = binary
        evidence.update(
            {
                "status": "PASS",
                "returncode": 0,
                "binary": str(binary),
                "reason": f"built {binary.name}",
            }
        )
    else:
        RUNTIME_CONTEXT["adapter_binary"] = None
        evidence.update(
            {
                "status": "FAIL",
                "returncode": completed.returncode,
                "reason": "build failed or produced no adapter binary",
            }
        )
    return evidence


def execute_check(definition: EvidenceDefinition, log_path: Path) -> dict[str, Any]:
    assert definition.name in CHECKS
    evidence: dict[str, Any] = {
        "package": None,
        "test": definition.name,
        "command": ["runner", definition.name],
    }
    try:
        detail = CHECKS[definition.name]()
    except GateConfigurationError as error:
        evidence.update({"status": "FAIL", "returncode": 1, "reason": str(error)})
        log_path.write_text(str(error) + "\n", encoding="utf-8")
    except (OSError, KeyError) as error:
        evidence.update({"status": "BLOCKED", "reason": str(error)})
        log_path.write_text(str(error) + "\n", encoding="utf-8")
    except Exception as error:  # fail closed on unexpected failures
        evidence.update({"status": "FAIL", "returncode": 1, "reason": f"unexpected: {error!r}"})
        log_path.write_text(repr(error) + "\n", encoding="utf-8")
    else:
        evidence.update({"status": "PASS", "returncode": 0, "reason": detail})
        log_path.write_text(detail + "\n", encoding="utf-8")
    return evidence


def execute_definition(definition: EvidenceDefinition, logs: Path, index: int) -> dict[str, Any]:
    log_path = logs / f"{index:03d}-{_slug(definition.name)}.log"
    if definition.kind == "rust":
        evidence = execute_rust_exact(definition, log_path)
    elif definition.kind == "rust_package":
        evidence = execute_rust_package(definition, log_path)
    elif definition.kind == "python":
        evidence = execute_python_file(definition, log_path)
    elif definition.kind == "build":
        evidence = execute_build(definition, log_path)
    elif definition.kind == "check":
        evidence = execute_check(definition, log_path)
    else:  # pragma: no cover - manifest validation forbids unknown kinds
        raise GateConfigurationError(f"unknown evidence kind {definition.kind!r}")
    evidence.update(
        {
            "kind": definition.kind,
            "surface": definition.surface,
            "log": f"logs/{log_path.name}",
        }
    )
    return evidence


# Gate-B python scenario nodes drive the REAL adapter binary through
# MTGML_M2_ADAPTER_BIN; the unit/guard suites do not need it.


def aggregate(statuses: Iterable[str]) -> str:
    """FAIL-dominant aggregation: FAIL outranks BLOCKED outranks NOT_RUN."""
    values = set(statuses)
    if "FAIL" in values:
        return "FAIL"
    if "BLOCKED" in values:
        return "BLOCKED"
    if "NOT_RUN" in values:
        return "NOT_RUN"
    return "PASS"


def render_markdown(report: dict[str, Any]) -> str:
    lines = [
        "# M2.H Gate Verification",
        "",
        "Generated outside the reproducible source archive by `scripts/run_m2_h_gates.py`.",
        "",
        f"- Mode: **{report['mode']}**",
        f"- Source commit: `{report.get('source_commit')}`",
    ]
    for gate in report["gates"]:
        lines.append(f"- `{gate['name']}`: **{gate['gate_status']}**")
    lines.extend(["", "| Evidence | Status | Surface |", "|---|---:|---|"])
    for gate in report["gates"]:
        for item in gate["evidence"]:
            lines.append(f"| `{item['test']}` | **{item['status']}** | {item['surface']} |")
    lines.append("")
    return "\n".join(lines)


def _startup_validations() -> None:
    verify_player_surface_closure()
    verify_schema_contract_digests()
    verify_registry_relation()
    verify_variant_closures()


def _print_schema_digests() -> None:
    for name in TOP_LEVEL_PLAYER_SCHEMAS:
        contract = name.removesuffix(".schema.json")
        print(json.dumps({"contract": contract, "digest": schema_contract_digest(name)}))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--development", action="store_true")
    parser.add_argument("--output-dir", type=Path, default=OUTPUT)
    parser.add_argument("--expect-commit", metavar="SHA", default=None)
    parser.add_argument(
        "--print-schema-digests",
        action="store_true",
        help="print current SchemaContractDigest values for re-pinning and exit",
    )
    args = parser.parse_args()

    if args.print_schema_digests:
        _print_schema_digests()
        return 0

    # Fail closed on configuration drift BEFORE creating output or evidence.
    try:
        _startup_validations()
    except GateConfigurationError as error:
        print(f"CONFIGURATION ERROR (no evidence executed): {error}")
        return 2

    output = args.output_dir.resolve()
    try:
        logs = prepare_output(output)
    except (OSError, RuntimeError, ValueError) as error:
        print(f"BLOCKED: {error}")
        return 2

    before = source_snapshot()
    toolchains = toolchain_snapshot()

    gates: list[dict[str, Any]] = []
    log_index = 0
    for gate_name, definitions in GATE_TESTS.items():
        evidence = []
        for definition in definitions:
            log_index += 1
            evidence.append(execute_definition(definition, logs, log_index))
        underlying = aggregate(item["status"] for item in evidence)
        gates.append({"name": gate_name, "underlying": underlying, "evidence": evidence})

    after = source_snapshot()
    if before.get("clean") and after.get("clean"):
        unchanged = (
            before.get("commit") == after.get("commit")
            and before.get("tree") == after.get("tree")
            and before.get("fingerprint") == after.get("fingerprint")
        )
        source_identity_status = "PASS" if unchanged else "FAIL"
    else:
        source_identity_status = "BLOCKED" if not before.get("clean") else "FAIL"

    expected_commit_note = None
    if (
        args.expect_commit
        and not args.development
        and source_identity_status == "PASS"
        and before.get("commit") != args.expect_commit
    ):
        source_identity_status = "FAIL"
        expected_commit_note = (
            f"source head {before.get('commit')} does not equal the "
            f"expected target SHA {args.expect_commit}"
        )

    for gate in gates:
        if args.development:
            gate["gate_status"] = "NOT_RUN"
        elif source_identity_status != "PASS":
            gate["gate_status"] = source_identity_status
        else:
            gate["gate_status"] = aggregate(
                (gate["underlying"], toolchains.get("status", "BLOCKED"))
            )

    overall_gate = aggregate(gate["gate_status"] for gate in gates)
    report = {
        "generated_at": datetime.now(UTC).isoformat(),
        "mode": "development" if args.development else "authoritative",
        "milestone": "M2.H",
        "reporter": "scripts/run_m2_h_gates.py",
        "source_commit": before.get("commit"),
        "expected_commit": args.expect_commit,
        "expected_commit_note": expected_commit_note,
        "source_tree_identity": {
            "status": source_identity_status,
            "before": before,
            "after": after,
        },
        "toolchains": toolchains,
        "gates": [
            {key: value for key, value in gate.items() if key != "underlying"} for gate in gates
        ],
        "host": {
            "platform": platform.platform(),
            "node": platform.node(),
            "python": sys.executable,
        },
    }
    (output / "m2-h-gate-results.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (output / "M2_H_GATES.md").write_text(render_markdown(report), encoding="utf-8")
    print(
        json.dumps(
            {
                "mode": report["mode"],
                "milestone": report["milestone"],
                "source_commit": report["source_commit"],
                "source_identity": source_identity_status,
                "gates": {gate["name"]: gate["gate_status"] for gate in gates},
                "overall": overall_gate,
                "output_dir": str(output),
            },
            sort_keys=True,
        )
    )
    if args.development:
        underlying_all = aggregate(gate["underlying"] for gate in gates)
        return 0 if underlying_all == "PASS" and toolchains.get("status") == "PASS" else 2
    return 0 if overall_gate == "PASS" else 2


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except GateConfigurationError as error:
        print(f"CONFIGURATION ERROR (no evidence executed): {error}")
        raise SystemExit(2) from None
