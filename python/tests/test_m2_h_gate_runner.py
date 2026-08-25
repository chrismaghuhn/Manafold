"""H.7-ii self-tests over ``scripts/run_m2_h_gates.py`` (Issue #55).

Import-level and pure-function coverage of the M2.H gate runner's startup
authorities and evidence parsers. The runner module is imported directly
(the established ``test_m2_d_gate_runner`` pattern); NO evidence subprocess
is ever spawned — ``run_command`` is stubbed at its boundary — and no
repository file is mutated: synthetic sources live in per-test temporary
directories outside the repo.

Covered groups:
1. exact-set gate-manifest validation (current + four mutations),
2. file-level pytest count-parser integrity,
3. cargo whole-package result parser,
4. SchemaContractDigest determinism/drift/annotation-insensitivity,
5. mechanical player-surface extractors against synthetic sources,
6. decoder-registry set-relation validator directions,
7. FAIL-dominant status aggregation,
8. source-head snapshot and tracked-source fingerprint stability.
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from collections.abc import Iterable, Iterator, Sequence
from contextlib import contextmanager
from dataclasses import replace
from pathlib import Path
from typing import Any
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import run_m2_h_gates as runner


def _completed(returncode: int, output: str) -> subprocess.CompletedProcess[str]:
    return subprocess.CompletedProcess(args=[], returncode=returncode, stdout=output)


@contextmanager
def _stubbed_execution(returncode: int, output: str) -> Iterator[dict[str, Any]]:
    seen: dict[str, Any] = {}

    def fake_run(
        command: Sequence[str], extra_env: dict[str, str] | None = None
    ) -> subprocess.CompletedProcess[str]:
        seen["command"] = list(command)
        seen["extra_env"] = extra_env
        return _completed(returncode, output)

    with (
        mock.patch.object(runner, "run_command", fake_run),
        mock.patch.object(runner, "command_available", lambda command: True),
    ):
        yield seen


@contextmanager
def _log_path() -> Iterator[Path]:
    with tempfile.TemporaryDirectory() as tmp:
        yield Path(tmp) / "000-node.log"


# ---------------------------------------------------------------------------
# 2+3: executor parsers (run_command stubbed; nothing spawns).
# ---------------------------------------------------------------------------


class PythonSummaryParserTests(unittest.TestCase):
    """execute_python_file accepts exactly N passed with zero substitutes."""

    def _evidence(
        self, expected: int, output: str, returncode: int = 0
    ) -> tuple[dict[str, Any], dict[str, Any]]:
        definition = runner.python_file("python/tests/example.py", "example surface", expected)
        with _stubbed_execution(returncode, output) as seen, _log_path() as log:
            evidence = runner.execute_python_file(definition, log)
        return evidence, seen

    def test_exact_pass_count_is_accepted(self) -> None:
        evidence, seen = self._evidence(2, "======================= 2 passed in 0.06s =======\n")
        self.assertEqual(evidence["status"], "PASS")
        self.assertEqual(evidence["tests_observed"], 2)
        self.assertEqual(evidence["returncode"], 0)
        # Skeleton deviation Q.5: ONE file-level invocation under a single -v.
        self.assertEqual(seen["command"][1:], ["-m", "pytest", "-v", "python/tests/example.py"])

    def test_warnings_are_tracked_but_not_substitutes(self) -> None:
        evidence, _ = self._evidence(2, "2 passed, 1 warning in 0.10s\n")
        self.assertEqual(evidence["status"], "PASS")
        self.assertEqual(evidence["warnings"], 1)

    def test_every_substitute_outcome_is_rejected_even_with_exit_zero(self) -> None:
        for tail in ("1 skipped", "1 xfailed", "1 xpassed", "3 deselected"):
            with self.subTest(substitute=tail):
                evidence, _ = self._evidence(2, f"2 passed, {tail} in 0.02s\n")
                self.assertEqual(evidence["status"], "FAIL")
                self.assertIn("substitute", evidence["reason"])

    def test_failed_and_error_outcomes_are_rejected(self) -> None:
        for summary, code in (
            ("1 failed, 2 passed in 0.30s", 1),
            ("2 passed, 1 error in 0.20s", 1),
        ):
            with self.subTest(summary=summary):
                evidence, _ = self._evidence(2, f"{summary}\n", returncode=code)
                self.assertEqual(evidence["status"], "FAIL")

    def test_wrong_pass_count_is_rejected(self) -> None:
        evidence, _ = self._evidence(2, "3 passed in 0.05s\n")
        self.assertEqual(evidence["status"], "FAIL")

    def test_two_summary_lines_are_rejected(self) -> None:
        output = "====== 2 passed in 0.10s ======\n\n====== 2 passed in 0.11s ======\n"
        evidence, _ = self._evidence(2, output)
        self.assertEqual(evidence["status"], "FAIL")
        self.assertIn("across 2 summary lines", evidence["reason"])

    def test_missing_summary_is_rejected(self) -> None:
        evidence, _ = self._evidence(
            2, ".                                                            [100%]\n"
        )
        self.assertEqual(evidence["status"], "FAIL")
        self.assertIn("across 0 summary lines", evidence["reason"])

    def test_nonzero_returncode_is_rejected_despite_clean_summary(self) -> None:
        evidence, _ = self._evidence(2, "2 passed in 0.06s\n", returncode=1)
        self.assertEqual(evidence["status"], "FAIL")


class CargoPackageParserTests(unittest.TestCase):
    """execute_rust_package sums every all-ok result line to the exact pin."""

    def _evidence(
        self,
        results: list[tuple[str, int, int]],
        expected: int = runner.EXPECTED_ADAPTER_PACKAGE_PASSED,
        returncode: int = 0,
    ) -> dict[str, Any]:
        blocks = [
            f"running {passed + failed} tests\n"
            f"test result: {verdict}. {passed} passed; {failed} failed; "
            "0 ignored; 0 measured; 0 filtered out\n"
            for verdict, passed, failed in results
        ]
        definition = runner.rust_package(
            runner.ADAPTER_PACKAGE, runner.CARGO_PACKAGE_ADAPTER, "package surface", expected
        )
        with _stubbed_execution(returncode, "".join(blocks)), _log_path() as log:
            return runner.execute_rust_package(definition, log)

    def test_multiple_ok_lines_sum_to_the_pinned_total(self) -> None:
        evidence = self._evidence([("ok", 13, 0), ("ok", 8, 0)])
        self.assertEqual(evidence["status"], "PASS")
        self.assertEqual(evidence["tests_observed"], runner.EXPECTED_ADAPTER_PACKAGE_PASSED)

    def test_any_failed_verdict_is_rejected(self) -> None:
        evidence = self._evidence([("ok", 21, 0), ("FAILED", 0, 3)])
        self.assertEqual(evidence["status"], "FAIL")

    def test_total_mismatch_is_rejected(self) -> None:
        evidence = self._evidence([("ok", 13, 0), ("ok", 7, 0)])
        self.assertEqual(evidence["status"], "FAIL")
        self.assertIn("summary mismatch", evidence["reason"])

    def test_nonzero_returncode_is_rejected_despite_matching_counts(self) -> None:
        evidence = self._evidence([("ok", 13, 0), ("ok", 8, 0)], returncode=1)
        self.assertEqual(evidence["status"], "FAIL")


# ---------------------------------------------------------------------------
# 1: exact-set gate-manifest validation.
# ---------------------------------------------------------------------------


def _copied_manifests() -> tuple[dict[str, Any], dict[str, Any]]:
    return (
        {gate: tuple(defs) for gate, defs in runner.GATE_TESTS.items()},
        {gate: tuple(expected) for gate, expected in runner.EXPECTED_EVIDENCE.items()},
    )


class GateManifestExactnessTests(unittest.TestCase):
    """validate_gate_manifest pins the exact ordered node set per gate."""

    def test_current_constants_validate(self) -> None:
        runner.validate_gate_manifest()

    def test_duplicate_node_within_one_gate_is_rejected(self) -> None:
        gates, expected = _copied_manifests()
        wire = runner.GATE_WIRE_PARITY
        gates[wire] = (*gates[wire], gates[wire][0])
        with (
            mock.patch.object(runner, "GATE_TESTS", gates),
            mock.patch.object(runner, "EXPECTED_EVIDENCE", expected),
            self.assertRaises(runner.GateConfigurationError) as raised,
        ):
            runner.validate_gate_manifest()
        self.assertIn("duplicate evidence node registration", str(raised.exception))

    def test_renamed_node_reports_missing_and_extra(self) -> None:
        gates, expected = _copied_manifests()
        wire = runner.GATE_WIRE_PARITY
        original = gates[wire][0]
        gates[wire] = (replace(original, name="bogus::renamed"), *gates[wire][1:])
        with (
            mock.patch.object(runner, "GATE_TESTS", gates),
            mock.patch.object(runner, "EXPECTED_EVIDENCE", expected),
            self.assertRaises(runner.GateConfigurationError) as raised,
        ):
            runner.validate_gate_manifest()
        message = str(raised.exception)
        self.assertIn("evidence manifest drift", message)
        self.assertIn(f"missing=['{original.name}']", message)
        self.assertIn("extra=['bogus::renamed']", message)

    def test_reordered_nodes_are_rejected(self) -> None:
        gates, expected = _copied_manifests()
        wire = runner.GATE_WIRE_PARITY
        gates[wire] = (gates[wire][1], gates[wire][0], *gates[wire][2:])
        with (
            mock.patch.object(runner, "GATE_TESTS", gates),
            mock.patch.object(runner, "EXPECTED_EVIDENCE", expected),
            self.assertRaises(runner.GateConfigurationError) as raised,
        ):
            runner.validate_gate_manifest()
        self.assertIn("order must match EXPECTED_EVIDENCE", str(raised.exception))

    def test_same_node_registered_across_both_gates_is_rejected(self) -> None:
        gates, expected = _copied_manifests()
        donor = gates[runner.GATE_WIRE_PARITY][0]
        other = runner.GATE_ADAPTER_PARITY
        gates[other] = (*gates[other], donor)
        expected[other] = (*expected[other], donor.name)
        with (
            mock.patch.object(runner, "GATE_TESTS", gates),
            mock.patch.object(runner, "EXPECTED_EVIDENCE", expected),
            self.assertRaises(runner.GateConfigurationError) as raised,
        ):
            runner.validate_gate_manifest()
        self.assertIn("duplicate evidence node registered across gates", str(raised.exception))


# ---------------------------------------------------------------------------
# 4: SchemaContractDigest normalizer identities.
# ---------------------------------------------------------------------------

_DIGEST_BASE: dict[str, Any] = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "required": ["a"],
    "properties": {"a": {"type": "string", "maxLength": 5}},
}


def _digest_of(schema: dict[str, Any]) -> str:
    with mock.patch.object(runner, "_load_schema", lambda name: schema):
        return runner.schema_contract_digest("synthetic.schema.json")


class SchemaContractDigestTests(unittest.TestCase):
    """Digests are deterministic, annotation-blind, and field-drift-sensitive."""

    def test_same_input_yields_the_same_stable_hexdigest(self) -> None:
        first = _digest_of(_DIGEST_BASE)
        second = _digest_of(_DIGEST_BASE)
        self.assertEqual(first, second)
        self.assertRegex(first, r"^[0-9a-f]{64}$")

    def test_annotation_only_changes_keep_the_digest_identical(self) -> None:
        annotated = {
            **_DIGEST_BASE,
            "title": "Synthetic",
            "description": "documentation only",
            "$comment": "notes",
            "examples": [{"a": "x"}],
            "default": {},
            "properties": {
                **_DIGEST_BASE["properties"],
                "a": {
                    **_DIGEST_BASE["properties"]["a"],
                    "title": "A",
                    "description": "documented",
                    "examples": ["x"],
                },
            },
        }
        self.assertEqual(_digest_of(annotated), _digest_of(_DIGEST_BASE))

    def test_added_optional_property_changes_the_digest(self) -> None:
        widened = {
            **_DIGEST_BASE,
            "properties": {**_DIGEST_BASE["properties"], "b": {"type": "integer"}},
        }
        self.assertNotEqual(_digest_of(widened), _digest_of(_DIGEST_BASE))

    def test_changed_bound_changes_the_digest(self) -> None:
        tightened = {
            **_DIGEST_BASE,
            "properties": {**_DIGEST_BASE["properties"], "a": {"type": "string", "maxLength": 4}},
        }
        self.assertNotEqual(_digest_of(tightened), _digest_of(_DIGEST_BASE))


# ---------------------------------------------------------------------------
# 5: mechanical player-surface extractors over synthetic sources.
# ---------------------------------------------------------------------------

ENDPOINT_RS_PINNED = """\
pub struct Ctx;

pub trait PlayerEndpoint {
    fn perspective(&self) -> PlayerId;
    fn observation(&self) -> Result<ObservationEnvelope, PlayerEndpointError>;
    fn information_state(&self) -> Result<PlayerInformationStateV2, PlayerEndpointError>;
    fn visible_decision(&self) -> Result<Option<PlayerDecisionRequestV2>, PlayerEndpointError>;
    fn submit(&self, response: DecisionResponseV2) -> Result<PlayerStepV2, PlayerEndpointError>;
}

impl Ctx {
    pub fn unrelated(&self) -> u8 {
        0
    }
}
"""

BOUNDARY_RS_PINNED = """\
pub enum PlayerBoundaryError {
    Wire(PlayerWireErrorCodeV1),
    Service(PlayerServiceErrorCodeV1),
}
"""

PLAYER_CLIENT_PY_PINNED = """\
from typing import Protocol


class PlayerClient(Protocol):
    def observation(self) -> ObservationEnvelope: ...

    def information_state(self) -> PlayerInformationStateV2: ...

    def visible_decision(self) -> PlayerDecisionRequestV2 | None: ...

    def submit(self, response: DecisionResponseV2) -> PlayerStepV2: ...
"""

ADAPTER_CLIENT_PY_PINNED = """\
class AdapterPlayerClient:
    def __init__(self) -> None:
        self._handle = object()

    def _send(self, payload: bytes) -> bytes:
        return payload

    def observation(self): ...

    def information_state(self): ...

    def visible_decision(self): ...

    def submit(self, response): ...
"""


@contextmanager
def _synthetic_surface(
    endpoint_rs: str, boundary_rs: str, protocol_py: str, adapter_py: str
) -> Iterator[None]:
    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)
        endpoint = base / "endpoint.rs"
        boundary = base / "boundary.rs"
        player_client = base / "player_client.py"
        adapter_client = base / "adapter_client.py"
        endpoint.write_text(endpoint_rs, encoding="utf-8")
        boundary.write_text(boundary_rs, encoding="utf-8")
        player_client.write_text(protocol_py, encoding="utf-8")
        adapter_client.write_text(adapter_py, encoding="utf-8")
        with (
            mock.patch.object(runner, "ENDPOINT_RS", endpoint),
            mock.patch.object(runner, "BOUNDARY_RS", boundary),
            mock.patch.object(runner, "PLAYER_CLIENT_PY", player_client),
            mock.patch.object(runner, "ADAPTER_CLIENT_PY", adapter_client),
        ):
            yield


class PlayerSurfaceExtractorTests(unittest.TestCase):
    """Extractors reproduce the pinned closure and fail closed on drift."""

    def test_extractors_reproduce_the_pinned_sets(self) -> None:
        with _synthetic_surface(
            ENDPOINT_RS_PINNED,
            BOUNDARY_RS_PINNED,
            PLAYER_CLIENT_PY_PINNED,
            ADAPTER_CLIENT_PY_PINNED,
        ):
            # The rust extractor whitespace-normalizes signatures, exactly as
            # verify_player_surface_closure normalizes the pins before comparing.
            normalized_rust = {
                name: {
                    "params": {
                        param: runner._norm_type(value)
                        for param, value in signature["params"].items()
                    },
                    "returns": runner._norm_type(str(signature["returns"])),
                }
                for name, signature in runner.RUST_PLAYER_ENDPOINT_METHODS.items()
            }
            self.assertEqual(runner.extract_rust_trait_methods("t"), normalized_rust)
            self.assertEqual(
                runner.extract_rust_enum_variants("t"),
                frozenset(map(runner._norm_type, runner.RUST_PLAYER_BOUNDARY_VARIANTS)),
            )
            self.assertEqual(
                runner.extract_python_protocol_methods("t"), runner.PYTHON_PROTOCOL_METHODS
            )
            self.assertEqual(
                runner.extract_adapter_public_methods("t"), runner.ADAPTER_PUBLIC_METHODS
            )

    def test_closure_check_passes_on_the_pinned_shape(self) -> None:
        with _synthetic_surface(
            ENDPOINT_RS_PINNED,
            BOUNDARY_RS_PINNED,
            PLAYER_CLIENT_PY_PINNED,
            ADAPTER_CLIENT_PY_PINNED,
        ):
            detail = runner.verify_player_surface_closure()
        self.assertTrue(detail.startswith("closure holds:"), detail)

    def test_removed_rust_method_fails_closed_against_the_pinned_set(self) -> None:
        removed = ENDPOINT_RS_PINNED.replace(
            "    fn visible_decision(&self) -> "
            "Result<Option<PlayerDecisionRequestV2>, PlayerEndpointError>;\n",
            "",
        )
        self.assertNotEqual(removed, ENDPOINT_RS_PINNED)
        with (
            _synthetic_surface(
                removed, BOUNDARY_RS_PINNED, PLAYER_CLIENT_PY_PINNED, ADAPTER_CLIENT_PY_PINNED
            ),
            self.assertRaises(runner.GateConfigurationError) as raised,
        ):
            runner.verify_player_surface_closure()
        self.assertIn("PlayerEndpoint trait drift", str(raised.exception))

    def test_renamed_python_protocol_method_fails_closed(self) -> None:
        renamed = PLAYER_CLIENT_PY_PINNED.replace("def observation(", "def peek_observation(")
        self.assertNotEqual(renamed, PLAYER_CLIENT_PY_PINNED)
        with (
            _synthetic_surface(
                ENDPOINT_RS_PINNED, BOUNDARY_RS_PINNED, renamed, ADAPTER_CLIENT_PY_PINNED
            ),
            self.assertRaises(runner.GateConfigurationError) as raised,
        ):
            runner.verify_player_surface_closure()
        self.assertIn("PlayerClient protocol drift", str(raised.exception))


# ---------------------------------------------------------------------------
# 6: decoder-registry set relation (drift regression, never three-way).
# ---------------------------------------------------------------------------


def _decode_arms(names: Iterable[str]) -> str:
    return "\n".join(f'        "{name}" => Ok(decode_variant()),' for name in sorted(names))


def _wire_lib_rs(names: Iterable[str], extra_arm: str | None = None) -> str:
    arms = _decode_arms(names)
    if extra_arm is not None:
        arms += f'\n        "{extra_arm}" => Ok(decode_variant()),'
    return (
        "fn decode_named(raw: &[u8]) -> Result<Box<dyn Named>, Unknown> {\n"
        "    let name = core::str::from_utf8(raw)?;\n"
        "    match name {\n"
        f"{arms}\n"
        "        _ => Err(Unknown),\n"
        "    }\n"
        "}\n"
        "\n"
        "#[derive(Debug, Error)]\n"
        "pub enum DecodeError {}\n"
    )


def _decoders_py(names: Iterable[str]) -> str:
    entries = "\n".join(f'    "{name}": _decode,' for name in sorted(names))
    return f"_DECODERS = {{\n{entries}\n}}\n"


def _validate_schemas_py(names: Iterable[str]) -> str:
    entries = "\n".join(f'    "{name}": "{name}.schema.json",' for name in sorted(names))
    return f"WIRE_MAPPING = {{\n{entries}\n}}\n"


@contextmanager
def _synthetic_registry(
    wire_lib_rs: str, decoders_py: str, validate_schemas_py: str
) -> Iterator[None]:
    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)
        wire_lib = base / "lib.rs"
        wire_py = base / "wire.py"
        validate_schemas = base / "validate_schemas.py"
        wire_lib.write_text(wire_lib_rs, encoding="utf-8")
        wire_py.write_text(decoders_py, encoding="utf-8")
        validate_schemas.write_text(validate_schemas_py, encoding="utf-8")
        with (
            mock.patch.object(runner, "WIRE_LIB_RS", wire_lib),
            mock.patch.object(runner, "WIRE_PY", wire_py),
            mock.patch.object(runner, "VALIDATE_SCHEMAS_PY", validate_schemas),
        ):
            yield


class RegistryRelationTests(unittest.TestCase):
    """rust == COMMON, python == COMMON + exception, schemas == COMMON."""

    def test_relation_holds_on_the_pinned_sets(self) -> None:
        common = runner.COMMON_NAMED_CONTRACTS
        python_set = common | runner.PYTHON_MECHANICAL_ONLY
        with _synthetic_registry(
            _wire_lib_rs(common), _decoders_py(python_set), _validate_schemas_py(common)
        ):
            self.assertEqual(runner.extract_rust_decode_named_contracts("t"), common)
            detail = runner.verify_registry_relation()
        self.assertTrue(detail.startswith("relation holds:"), detail)

    def test_python_missing_the_digest_input_exception_entry_fails(self) -> None:
        common = runner.COMMON_NAMED_CONTRACTS
        with (
            _synthetic_registry(
                _wire_lib_rs(common), _decoders_py(common), _validate_schemas_py(common)
            ),
            self.assertRaises(runner.GateConfigurationError) as raised,
        ):
            runner.verify_registry_relation()
        message = str(raised.exception)
        self.assertIn("python _DECODERS != COMMON union PYTHON_MECHANICAL_ONLY", message)
        self.assertIn("'information-state-digest-input.v2'", message)

    def test_rust_gaining_an_extra_arm_fails_against_pinned_common(self) -> None:
        common = runner.COMMON_NAMED_CONTRACTS
        python_set = common | runner.PYTHON_MECHANICAL_ONLY
        with (
            _synthetic_registry(
                _wire_lib_rs(common, extra_arm="bonus-contract.v9"),
                _decoders_py(python_set),
                _validate_schemas_py(common),
            ),
            self.assertRaises(runner.GateConfigurationError) as raised,
        ):
            runner.verify_registry_relation()
        self.assertIn("rust decode_named != COMMON", str(raised.exception))

    def test_schema_mapping_drift_fails(self) -> None:
        common = runner.COMMON_NAMED_CONTRACTS
        python_set = common | runner.PYTHON_MECHANICAL_ONLY
        drifted = common - {"episode-status.v1"}
        with (
            _synthetic_registry(
                _wire_lib_rs(common), _decoders_py(python_set), _validate_schemas_py(drifted)
            ),
            self.assertRaises(runner.GateConfigurationError) as raised,
        ):
            runner.verify_registry_relation()
        self.assertIn("schemas WIRE_MAPPING != COMMON", str(raised.exception))


# ---------------------------------------------------------------------------
# 7: FAIL-dominant aggregation.
# ---------------------------------------------------------------------------


class AggregationTests(unittest.TestCase):
    """FAIL outranks BLOCKED outranks NOT_RUN; only all-PASS aggregates PASS."""

    def test_fail_dominates_not_run(self) -> None:
        self.assertEqual(runner.aggregate(["FAIL", "NOT_RUN"]), "FAIL")
        self.assertEqual(runner.aggregate(["NOT_RUN", "FAIL"]), "FAIL")

    def test_blocked_dominates_not_run_without_masking_failures(self) -> None:
        self.assertEqual(runner.aggregate(["BLOCKED", "NOT_RUN"]), "BLOCKED")
        self.assertNotEqual(runner.aggregate(["BLOCKED", "NOT_RUN"]), "PASS")

    def test_all_pass_aggregates_pass(self) -> None:
        self.assertEqual(runner.aggregate(["PASS", "PASS", "PASS"]), "PASS")

    def test_remaining_rankings(self) -> None:
        self.assertEqual(runner.aggregate(["PASS", "NOT_RUN"]), "NOT_RUN")
        self.assertEqual(runner.aggregate([]), "PASS")
        self.assertEqual(runner.aggregate(["FAIL", "BLOCKED"]), "FAIL")


# ---------------------------------------------------------------------------
# 8: source-head snapshot and fingerprint stability (read-only git).
# ---------------------------------------------------------------------------


def _independent_head() -> str:
    completed = subprocess.run(
        ("git", "rev-parse", "HEAD"),
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    return completed.stdout.strip()


class SourceHeadFingerprintTests(unittest.TestCase):
    """source_snapshot describes this clean checkout at HEAD, stably."""

    def test_clean_tree_reports_clean_true_at_head_commit(self) -> None:
        snapshot = runner.source_snapshot()
        self.assertTrue(snapshot["clean"], snapshot.get("git_status"))
        self.assertEqual(snapshot["commit"], _independent_head())

    def test_tracked_source_fingerprint_is_stable_across_calls(self) -> None:
        first = runner.source_snapshot()
        second = runner.source_snapshot()
        self.assertTrue(first["clean"], first.get("git_status"))
        self.assertTrue(second["clean"], second.get("git_status"))
        self.assertEqual(first["fingerprint"], second["fingerprint"])


if __name__ == "__main__":
    unittest.main()
