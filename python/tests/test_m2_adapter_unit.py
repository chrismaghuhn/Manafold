"""Unit tests for the experimental mtgml._m2_adapter package.

Everything here runs WITHOUT the Rust adapter binary: protocol framing
is exercised against tiny stdlib fake-child processes and the clients
against scripted in-memory transports.
"""

from __future__ import annotations

import base64
import gc
import inspect
import os
import re
import sys
import time
import typing
import unittest
import warnings
from collections.abc import Mapping
from pathlib import Path
from types import FunctionType
from typing import Any
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml._m2_adapter import (
    AdapterError,
    AdapterPlayerClient,
    RestrictedPlayerTransport,
    SyntheticEnvironmentClient,
)
from mtgml._m2_adapter.process import (
    BINARY_ENV_VAR,
    TRUSTED_KEY_ENV_VAR,
    SubprocessTransport,
    build_child_environment,
    generate_trusted_key,
)
from mtgml._m2_adapter.protocol import (
    INVALID_PARAMS,
    PARSE_ERROR,
    REQUEST_TIMEOUT,
    SERVICE_UNAVAILABLE,
    TRANSPORT_CLOSED,
    UNKNOWN_TOKEN,
)
from mtgml._m2_adapter.submission import (
    encode_decision_response_submission_v2 as encode_submission,
)
from mtgml.canonical import canonical_json_bytes as canonical_bytes
from mtgml.decision import (
    DECISION_RESPONSE_V2_SCHEMA,
    DecisionAnswerV2,
    DecisionResponseV2,
    PlayerDecisionRequestV2,
)
from mtgml.observation import (
    ObservationEnvelope,
    PlayerInformationStateV2,
    PlayerStepV2,
)
from mtgml.wire import decode_canonical

GOLDEN_DIR = ROOT / "wire" / "golden"


def golden_bytes(name: str) -> bytes:
    return (GOLDEN_DIR / name).read_bytes()


def wire_b64(raw: bytes) -> str:
    return base64.b64encode(raw).decode("ascii")


CHILD_ECHO = """\
import json
import sys

for line in sys.stdin.buffer:
    request = json.loads(line.decode("utf-8"))
    reply = {
        "v": 1,
        "id": request.get("id"),
        "ok": True,
        "result": {"echo_id": request.get("id"), "cmd": request.get("cmd")},
    }
    sys.stdout.buffer.write(json.dumps(reply).encode("utf-8") + b"\\n")
    sys.stdout.flush()
"""

CHILD_FAKE_ADAPTER = """\
import json
import sys

for line in sys.stdin.buffer:
    request = json.loads(line.decode("utf-8"))
    command = request.get("cmd")
    params = request.get("params") or {}
    if command == "bind_player":
        result = {"token": "tok-" + str(params.get("player"))}
    else:
        result = {
            "trusted_key_echo": params.get("trusted_key"),
            "params_echo": params,
        }
    reply = {"v": 1, "id": request.get("id"), "ok": True, "result": result}
    sys.stdout.buffer.write(json.dumps(reply).encode("utf-8") + b"\\n")
    sys.stdout.flush()
    if command == "shutdown":
        break
"""

CHILD_WRONG_ID = """\
import json
import sys

for line in sys.stdin.buffer:
    request = json.loads(line.decode("utf-8"))
    del request
    reply = {"v": 1, "id": 999, "ok": True, "result": {}}
    sys.stdout.buffer.write(json.dumps(reply).encode("utf-8") + b"\\n")
    sys.stdout.flush()
"""

CHILD_GARBAGE = """\
import sys

sys.stdin.buffer.readline()
sys.stdout.buffer.write(b"not-json\\n")
sys.stdout.flush()
"""

CHILD_NOT_OBJECT = """\
import sys

sys.stdin.buffer.readline()
sys.stdout.buffer.write(b"[1, 2]\\n")
sys.stdout.flush()
"""

CHILD_BAD_VERSION = """\
import sys

sys.stdin.buffer.readline()
sys.stdout.buffer.write(b'{"v": 2, "id": 1, "ok": true, "result": {}}\\n')
sys.stdout.flush()
"""

CHILD_BAD_OK = """\
import sys

sys.stdin.buffer.readline()
sys.stdout.buffer.write(b'{"v": 1, "id": 1, "ok": "yes", "result": {}}\\n')
sys.stdout.flush()
"""

CHILD_UNKNOWN_CODE = """\
import sys

sys.stdin.buffer.readline()
sys.stdout.buffer.write(
    b'{"v": 1, "id": 1, "ok": false, "error": {"code": "mystery"}}\\n'
)
sys.stdout.flush()
"""

CHILD_ERROR_THEN_OK = """\
import json
import sys

first = True
for line in sys.stdin.buffer:
    request = json.loads(line.decode("utf-8"))
    if first:
        reply = {
            "v": 1,
            "id": request.get("id"),
            "ok": False,
            "error": {"code": "service_unavailable"},
        }
        first = False
    else:
        reply = {
            "v": 1,
            "id": request.get("id"),
            "ok": True,
            "result": {"cmd": request.get("cmd")},
        }
    sys.stdout.buffer.write(json.dumps(reply).encode("utf-8") + b"\\n")
    sys.stdout.flush()
"""

CHILD_HANG = """\
import time
import sys

sys.stdin.buffer.readline()
time.sleep(30)
"""

CHILD_CRASH = """\
import os
import sys

sys.stdin.buffer.readline()
os._exit(7)
"""


def child_transport(script: str, **kwargs: Any) -> SubprocessTransport:
    return SubprocessTransport(argv=[sys.executable, "-c", script], **kwargs)


class FakeTransport:
    def __init__(self, *script: Any) -> None:
        self.script: list[Any] = list(script)
        self.calls: list[tuple[str, dict[str, Any]]] = []
        self.shutdown_marked = False

    @property
    def _trusted_key(self) -> str:
        return "fake-trusted-key"

    def call(self, command: str, params: dict[str, Any]) -> dict[str, Any]:
        self.calls.append((command, dict(params)))
        action = self.script.pop(0)
        if isinstance(action, Exception):
            raise action
        return action

    def _mark_shutdown(self) -> None:
        self.shutdown_marked = True


class FramingPairingTests(unittest.TestCase):
    def test_round_trip_pairs_ids_monotonically(self) -> None:
        transport = child_transport(CHILD_ECHO)
        try:
            first = transport.call("observation", {"token": "t"})
            second = transport.call("submit", {"token": "t"})
        finally:
            transport.close()
        self.assertEqual(first, {"echo_id": 1, "cmd": "observation"})
        self.assertEqual(second, {"echo_id": 2, "cmd": "submit"})

    def test_id_echo_mismatch_raises_parse_error(self) -> None:
        transport = child_transport(CHILD_WRONG_ID)
        try:
            with self.assertRaises(AdapterError) as ctx:
                transport.call("observation", {})
            self.assertEqual(ctx.exception.code, PARSE_ERROR)
        finally:
            transport.close()

    def test_malformed_frame_raises_parse_error(self) -> None:
        transport = child_transport(CHILD_GARBAGE)
        try:
            with self.assertRaises(AdapterError) as ctx:
                transport.call("observation", {})
            self.assertEqual(ctx.exception.code, PARSE_ERROR)
        finally:
            transport.close()

    def test_non_object_frame_raises_parse_error(self) -> None:
        transport = child_transport(CHILD_NOT_OBJECT)
        try:
            with self.assertRaises(AdapterError) as ctx:
                transport.call("observation", {})
            self.assertEqual(ctx.exception.code, PARSE_ERROR)
        finally:
            transport.close()

    def test_unsupported_version_raises_parse_error(self) -> None:
        transport = child_transport(CHILD_BAD_VERSION)
        try:
            with self.assertRaises(AdapterError) as ctx:
                transport.call("observation", {})
            self.assertEqual(ctx.exception.code, PARSE_ERROR)
        finally:
            transport.close()

    def test_non_boolean_ok_flag_raises_parse_error(self) -> None:
        transport = child_transport(CHILD_BAD_OK)
        try:
            with self.assertRaises(AdapterError) as ctx:
                transport.call("observation", {})
            self.assertEqual(ctx.exception.code, PARSE_ERROR)
        finally:
            transport.close()

    def test_unknown_error_code_raises_parse_error(self) -> None:
        transport = child_transport(CHILD_UNKNOWN_CODE)
        try:
            with self.assertRaises(AdapterError) as ctx:
                transport.call("observation", {})
            self.assertEqual(ctx.exception.code, PARSE_ERROR)
        finally:
            transport.close()

    def test_error_envelope_surfaces_code_and_keeps_transport_open(self) -> None:
        transport = child_transport(CHILD_ERROR_THEN_OK)
        try:
            with self.assertRaises(AdapterError) as ctx:
                transport.call("observation", {})
            self.assertEqual(ctx.exception.code, SERVICE_UNAVAILABLE)
            followup = transport.call("information_state", {})
            self.assertEqual(followup, {"cmd": "information_state"})
        finally:
            transport.close()


class TimeoutAndCrashTests(unittest.TestCase):
    def test_request_timeout_terminates_child_and_fails_closed(self) -> None:
        transport = child_transport(CHILD_HANG, timeout=0.4)
        try:
            started = time.monotonic()
            with self.assertRaises(AdapterError) as ctx:
                transport.call("observation", {})
            self.assertLess(time.monotonic() - started, 15.0)
            self.assertEqual(ctx.exception.code, REQUEST_TIMEOUT)
            with self.assertRaises(AdapterError) as followup:
                transport.call("observation", {})
            self.assertEqual(followup.exception.code, TRANSPORT_CLOSED)
        finally:
            transport.close()

    def test_child_crash_mid_request_raises_transport_closed(self) -> None:
        transport = child_transport(CHILD_CRASH)
        try:
            with self.assertRaises(AdapterError) as ctx:
                transport.call("observation", {})
            self.assertEqual(ctx.exception.code, TRANSPORT_CLOSED)
            with self.assertRaises(AdapterError) as followup:
                transport.call("observation", {})
            self.assertEqual(followup.exception.code, TRANSPORT_CLOSED)
        finally:
            transport.close()
        process = transport._process
        assert process is not None and process.stdin is not None and process.stdout is not None
        self.assertTrue(process.stdin.closed)
        self.assertTrue(process.stdout.closed)
        reader = transport._reader
        assert reader is not None
        self.assertFalse(reader.is_alive())


class BinaryResolutionTests(unittest.TestCase):
    def test_unset_binary_env_var_fails_closed_at_construction(self) -> None:
        env = os.environ.copy()
        env.pop(BINARY_ENV_VAR, None)
        with mock.patch.dict(os.environ, env, clear=True), self.assertRaises(AdapterError):
            SubprocessTransport()

    def test_nonexistent_binary_fails_closed_at_construction(self) -> None:
        missing = str(ROOT / "dist" / "definitely-not-a-binary-m2h.exe")
        patcher = mock.patch.dict(os.environ, {BINARY_ENV_VAR: missing})
        with patcher, self.assertRaises(AdapterError):
            SubprocessTransport()


class TrustedKeyIsolationTests(unittest.TestCase):
    def test_build_child_environment_copies_and_injects_key(self) -> None:
        key = generate_trusted_key()
        child = build_child_environment(key)
        self.assertIsNot(child, os.environ)
        self.assertEqual(child[TRUSTED_KEY_ENV_VAR], key)
        self.assertNotIn(TRUSTED_KEY_ENV_VAR, os.environ)

    def test_parent_environ_untouched_across_real_child_flow(self) -> None:
        before = dict(os.environ)
        transport = child_transport(CHILD_FAKE_ADAPTER)
        during_construction = dict(os.environ)
        child_env = dict(transport._child_environment)
        with SyntheticEnvironmentClient(transport) as environment:
            environment.reset_synthetic(players=("3", "4"), root_seed_hex="be" * 32)
            player = environment.bind_player("2")
            self.assertIsInstance(player, AdapterPlayerClient)
            self.assertEqual(player._token, "tok-2")
        after = dict(os.environ)
        self.assertEqual(before, during_construction)
        self.assertEqual(before, after)
        self.assertNotIn(TRUSTED_KEY_ENV_VAR, after)
        self.assertEqual(child_env[TRUSTED_KEY_ENV_VAR], transport._trusted_key)
        self.assertIsNotNone(re.fullmatch(r"[0-9a-f]{64}", transport._trusted_key))
        self.assertNotEqual(child_env, os.environ)

    def test_trusted_commands_carry_transport_scoped_key(self) -> None:
        fake = FakeTransport({}, {"token": "tok-5"})
        client = SyntheticEnvironmentClient(fake)
        client.reset_synthetic(root_seed_hex="ab" * 32)
        client.bind_player("5")
        self.assertEqual(len(fake.calls), 2)
        command, params = fake.calls[0]
        self.assertEqual(command, "reset_synthetic")
        self.assertEqual(params["trusted_key"], "fake-trusted-key")
        self.assertEqual(params["players"], ["1", "2"])
        self.assertEqual(params["root_seed_hex"], "ab" * 32)
        command, params = fake.calls[1]
        self.assertEqual(command, "bind_player")
        self.assertEqual(params["trusted_key"], "fake-trusted-key")
        self.assertEqual(params["player"], "5")

    def test_shutdown_marks_transport_closed_and_is_idempotent(self) -> None:
        fake = FakeTransport({})
        client = SyntheticEnvironmentClient(fake)
        client.shutdown()
        client.shutdown()
        self.assertTrue(fake.shutdown_marked)
        self.assertEqual(len(fake.calls), 1)


class PostShutdownTests(unittest.TestCase):
    def test_post_shutdown_write_never_touches_a_pipe(self) -> None:
        transport = child_transport("pass")
        transport._mark_shutdown()
        with self.assertRaises(AdapterError) as ctx:
            transport.call("observation", {})
        self.assertEqual(ctx.exception.code, TRANSPORT_CLOSED)
        self.assertIsNone(transport._process)

    def test_post_shutdown_write_after_real_session_fails_closed(self) -> None:
        transport = child_transport(CHILD_FAKE_ADAPTER)
        with SyntheticEnvironmentClient(transport) as environment:
            environment.reset_synthetic(root_seed_hex="11" * 32)
        with self.assertRaises(AdapterError) as ctx:
            transport.call("observation", {})
        self.assertEqual(ctx.exception.code, TRANSPORT_CLOSED)
        process = transport._process
        self.assertIsNotNone(process)
        assert process is not None
        self.assertIsNotNone(process.poll())
        self.assertEqual(process.poll(), 0)
        assert process.stdin is not None and process.stdout is not None
        self.assertTrue(process.stdin.closed)
        self.assertTrue(process.stdout.closed)


class DeterministicTeardownTests(unittest.TestCase):
    def test_full_spawn_use_teardown_emits_no_resource_warnings(self) -> None:
        with warnings.catch_warnings():
            warnings.simplefilter("error", ResourceWarning)
            transport = child_transport(CHILD_ECHO)
            result = transport.call("observation", {"token": "t"})
            transport.close()
            process = transport._process
            reader = transport._reader
            del transport, process, reader
            gc.collect()
        self.assertEqual(result, {"echo_id": 1, "cmd": "observation"})

    def test_close_closes_pipes_and_joins_reader_on_a_live_child(self) -> None:
        transport = child_transport(CHILD_ECHO)
        try:
            transport.call("observation", {"token": "t"})
        finally:
            transport.close()
        process = transport._process
        assert process is not None
        self.assertIsNotNone(process.poll())
        assert process.stdin is not None and process.stdout is not None
        self.assertTrue(process.stdin.closed)
        self.assertTrue(process.stdout.closed)
        reader = transport._reader
        assert reader is not None
        self.assertFalse(reader.is_alive())


class ApiInventoryTests(unittest.TestCase):
    def _instances(self) -> tuple[AdapterPlayerClient, SyntheticEnvironmentClient]:
        fake = FakeTransport({"token": "tok-x"})
        synthetic = SyntheticEnvironmentClient(fake)
        player = synthetic.bind_player("x")
        return player, synthetic

    def test_player_client_public_methods_are_exactly_the_protocol_four(self) -> None:
        player, _ = self._instances()
        public = {
            name
            for name, value in inspect.getmembers(player, inspect.ismethod)
            if not name.startswith("_")
        }
        self.assertEqual(public, {"observation", "information_state", "visible_decision", "submit"})

    def test_synthetic_client_public_methods_are_exactly_the_trusted_three(self) -> None:
        _, synthetic = self._instances()
        public = {
            name
            for name, value in inspect.getmembers(synthetic, inspect.ismethod)
            if not name.startswith("_")
        }
        self.assertEqual(public, {"reset_synthetic", "bind_player", "shutdown"})
        self.assertTrue(callable(synthetic.__enter__))
        self.assertTrue(callable(synthetic.__exit__))

    def test_synthetic_client_grows_only_the_underscore_private_direct_call(self) -> None:
        _, synthetic = self._instances()
        self.assertIn("_trusted_direct_call", dir(synthetic))
        attribute = SyntheticEnvironmentClient.__dict__["_trusted_direct_call"]
        self.assertIsInstance(attribute, FunctionType)
        self.assertTrue(attribute.__name__.startswith("_"))
        self.assertEqual(
            list(inspect.signature(attribute).parameters),
            ["self", "op", "player", "response_wire"],
        )
        hints = typing.get_type_hints(attribute)
        self.assertEqual(hints["op"], str)
        self.assertEqual(hints["player"], str)
        self.assertEqual(hints["response_wire"], bytes | None)
        self.assertEqual(hints["return"], Mapping[str, object])

    def test_no_generic_send_surface_on_either_client(self) -> None:
        player, synthetic = self._instances()
        for forbidden in ("send", "dispatch", "command", "_request"):
            self.assertNotIn(forbidden, dir(player))
            self.assertNotIn(forbidden, dir(synthetic))

    def test_restricted_seam_exposes_only_submit_wire_bytes(self) -> None:
        members = {
            name
            for name in dir(RestrictedPlayerTransport)
            if not name.startswith("__") and callable(getattr(RestrictedPlayerTransport, name))
        }
        self.assertEqual(members, {"_submit_wire_bytes"})

    def test_typed_signatures_match_the_playerclient_protocol(self) -> None:
        expectations: dict[str, dict[str, Any]] = {
            "observation": {"return": ObservationEnvelope},
            "information_state": {"return": PlayerInformationStateV2},
            "visible_decision": {"return": PlayerDecisionRequestV2 | None},
            "submit": {"response": DecisionResponseV2, "return": PlayerStepV2},
        }
        for method, expected in expectations.items():
            attribute = getattr(AdapterPlayerClient, method)
            self.assertIsInstance(attribute, FunctionType)
            self.assertEqual(typing.get_type_hints(attribute), expected)


class SubmissionEncoderTests(unittest.TestCase):
    def _response(self, answer: DecisionAnswerV2, **overrides: Any) -> DecisionResponseV2:
        return DecisionResponseV2(
            overrides.get("schema_version", DECISION_RESPONSE_V2_SCHEMA),
            overrides.get("player_decision_id", 5),
            overrides.get("state_revision", 7),
            answer,
        )

    def _encoded_dict(self, answer: dict[str, Any], **ids: Any) -> dict[str, Any]:
        return {
            "answer": answer,
            "player_decision_id": str(ids.get("player_decision_id", 5)),
            "schema_version": DECISION_RESPONSE_V2_SCHEMA,
            "state_revision": str(ids.get("state_revision", 7)),
        }

    def test_select_one_matches_hand_built_canonical_bytes(self) -> None:
        response = self._response(DecisionAnswerV2("select_one", candidate_id=3))
        encoded = encode_submission(response)
        expected = canonical_bytes(self._encoded_dict({"candidate_id": 3, "kind": "select_one"}))
        self.assertEqual(encoded, expected)

    def test_select_many_keeps_given_order_without_sorting_or_dedup(self) -> None:
        response = self._response(DecisionAnswerV2("select_many", candidate_ids=(4, 1, 4)))
        encoded = encode_submission(response)
        expected = canonical_bytes(
            self._encoded_dict({"candidate_ids": [4, 1, 4], "kind": "select_many"})
        )
        self.assertEqual(encoded, expected)

    def test_order_with_duplicate_ids_still_encodes(self) -> None:
        response = self._response(DecisionAnswerV2("order", candidate_ids=(2, 2)))
        encoded = encode_submission(response)
        expected = canonical_bytes(self._encoded_dict({"candidate_ids": [2, 2], "kind": "order"}))
        self.assertEqual(encoded, expected)

    def test_choose_number_encodes_negative_and_boundary_values(self) -> None:
        for value in (-(2**63), -5, 0, 2**63 - 1):
            response = self._response(DecisionAnswerV2("choose_number", value=value))
            encoded = encode_submission(response)
            expected = canonical_bytes(
                self._encoded_dict({"kind": "choose_number", "value": value})
            )
            self.assertEqual(encoded, expected)

    def test_empty_select_many_still_encodes(self) -> None:
        response = self._response(DecisionAnswerV2("select_many", candidate_ids=()))
        encoded = encode_submission(response)
        expected = canonical_bytes(self._encoded_dict({"candidate_ids": [], "kind": "select_many"}))
        self.assertEqual(encoded, expected)

    def test_out_of_range_candidate_count_still_encodes(self) -> None:
        response = self._response(DecisionAnswerV2("select_many", candidate_ids=tuple(range(64))))
        encoded = encode_submission(response)
        expected = canonical_bytes(
            self._encoded_dict({"candidate_ids": list(range(64)), "kind": "select_many"})
        )
        self.assertEqual(encoded, expected)

    def test_encoder_output_equals_checked_in_golden_bytes(self) -> None:
        response = DecisionResponseV2(
            DECISION_RESPONSE_V2_SCHEMA,
            1,
            0,
            DecisionAnswerV2("select_one", candidate_id=1),
        )
        fixture = golden_bytes("decision-response.v2-select-one.json")
        self.assertEqual(encode_submission(response), fixture)

    def test_structural_violations_raise_invalid_params(self) -> None:
        u32_bad = 2**32
        cases: list[DecisionResponseV2] = [
            self._response(DecisionAnswerV2("teleport")),
            self._response(DecisionAnswerV2("select_one")),
            self._response(DecisionAnswerV2("select_one", candidate_id=True)),
            self._response(DecisionAnswerV2("select_one", candidate_id=-1)),
            self._response(DecisionAnswerV2("select_one", candidate_id=u32_bad)),
            self._response(DecisionAnswerV2("choose_number", value=True)),
            self._response(DecisionAnswerV2("choose_number", value=2**63)),
            self._response(DecisionAnswerV2("select_many", candidate_ids=(0, u32_bad))),
            self._response(DecisionAnswerV2("select_one", candidate_id=1, candidate_ids=(1,))),
            self._response(DecisionAnswerV2("order", candidate_ids=(1,), value=0)),
            self._response(
                DecisionAnswerV2("select_one", candidate_id=1),
                schema_version="decision-response.v1",
            ),
            self._response(
                DecisionAnswerV2("select_one", candidate_id=1),
                player_decision_id=True,
            ),
            self._response(
                DecisionAnswerV2("select_one", candidate_id=1),
                player_decision_id=-1,
            ),
            self._response(
                DecisionAnswerV2("select_one", candidate_id=1),
                state_revision=2**64,
            ),
        ]
        for response in cases:
            with self.subTest(response=response):
                with self.assertRaises(AdapterError) as ctx:
                    encode_submission(response)
                self.assertEqual(ctx.exception.code, INVALID_PARAMS)


class ClientEndToEndTests(unittest.TestCase):
    def test_observation_decodes_through_the_real_codec(self) -> None:
        raw = golden_bytes("observation-envelope.v1.json")
        fake = FakeTransport({"observation_wire_b64": wire_b64(raw)})
        client = AdapterPlayerClient(fake, "tok-1")
        observation = client.observation()
        self.assertIsInstance(observation, ObservationEnvelope)
        self.assertEqual(observation, decode_canonical("observation-envelope.v1", raw))
        self.assertEqual(fake.calls, [("observation", {"token": "tok-1"})])

    def test_information_state_decodes_through_the_real_codec(self) -> None:
        raw = golden_bytes("information-state-envelope.v2.json")
        fake = FakeTransport({"information_state_wire_b64": wire_b64(raw)})
        client = AdapterPlayerClient(fake, "tok-1")
        state = client.information_state()
        self.assertIsInstance(state, PlayerInformationStateV2)
        self.assertEqual(state, decode_canonical("information-state-envelope.v2", raw))

    def test_visible_decision_returns_none_for_null_payload(self) -> None:
        fake = FakeTransport({"visible_decision_wire_b64": None})
        client = AdapterPlayerClient(fake, "tok-1")
        self.assertIsNone(client.visible_decision())

    def test_visible_decision_decodes_request_payload(self) -> None:
        raw = golden_bytes("player-decision-request.v2.json")
        fake = FakeTransport({"visible_decision_wire_b64": wire_b64(raw)})
        client = AdapterPlayerClient(fake, "tok-1")
        request = client.visible_decision()
        self.assertIsInstance(request, PlayerDecisionRequestV2)
        self.assertEqual(request, decode_canonical("player-decision-request.v2", raw))

    def test_visible_decision_missing_field_raises_parse_error(self) -> None:
        fake = FakeTransport({})
        client = AdapterPlayerClient(fake, "tok-1")
        with self.assertRaises(AdapterError) as ctx:
            client.visible_decision()
        self.assertEqual(ctx.exception.code, PARSE_ERROR)

    def test_typed_submit_end_to_end_through_scripted_step(self) -> None:
        step_raw = golden_bytes("player-step.v2.json")
        fake = FakeTransport({"step_wire_b64": wire_b64(step_raw)})
        client = AdapterPlayerClient(fake, "tok-7")
        response = DecisionResponseV2(
            DECISION_RESPONSE_V2_SCHEMA,
            1,
            0,
            DecisionAnswerV2("select_one", candidate_id=1),
        )
        step = client.submit(response)
        self.assertIsInstance(step, PlayerStepV2)
        self.assertEqual(step, decode_canonical("player-step.v2", step_raw))
        submitted = encode_submission(response)
        self.assertEqual(
            fake.calls,
            [("submit", {"token": "tok-7", "response_wire_b64": wire_b64(submitted)})],
        )

    def test_ok_false_codes_surface_verbatim(self) -> None:
        for code in ("malformed_response", "service_unavailable"):
            with self.subTest(code=code):
                fake = FakeTransport(AdapterError(code, "scripted failure"))
                client = AdapterPlayerClient(fake, "tok-1")
                response = DecisionResponseV2(
                    DECISION_RESPONSE_V2_SCHEMA,
                    1,
                    0,
                    DecisionAnswerV2("select_one", candidate_id=0),
                )
                with self.assertRaises(AdapterError) as ctx:
                    client.submit(response)
                self.assertEqual(ctx.exception.code, code)

    def test_bind_player_returns_client_holding_exactly_one_token(self) -> None:
        fake = FakeTransport({"token": "tok-9"})
        environment = SyntheticEnvironmentClient(fake)
        player = environment.bind_player("9")
        self.assertIsInstance(player, AdapterPlayerClient)
        self.assertEqual(player._token, "tok-9")


class RestrictedSeamTests(unittest.TestCase):
    def test_seam_forwards_arbitrary_bytes_and_returns_step_bytes(self) -> None:
        step_raw = golden_bytes("player-step.v2.json")
        fake = FakeTransport({"step_wire_b64": wire_b64(step_raw)})
        seam = RestrictedPlayerTransport(fake, "tok-7")
        raw = b"\xff\x00not-utf8\traw"
        out = seam._submit_wire_bytes("tok-7", raw)
        self.assertEqual(out, step_raw)
        self.assertEqual(
            fake.calls,
            [("submit", {"token": "tok-7", "response_wire_b64": wire_b64(raw)})],
        )

    def test_token_mismatch_answers_unknown_token_without_sending(self) -> None:
        fake = FakeTransport()
        seam = RestrictedPlayerTransport(fake, "tok-7")
        with self.assertRaises(AdapterError) as ctx:
            seam._submit_wire_bytes("tok-other", b"bytes")
        self.assertEqual(ctx.exception.code, UNKNOWN_TOKEN)
        self.assertEqual(fake.calls, [])


if __name__ == "__main__":
    unittest.main()
