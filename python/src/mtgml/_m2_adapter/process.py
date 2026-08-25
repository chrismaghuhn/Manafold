"""Capability-separated subprocess transports for the M2 semantic adapter.

Three classes share one child-process handle with strictly separated
capabilities:

``ProcessCore`` owns ONLY the mechanics: it spawns the adapter binary
located via ``MTGML_M2_ADAPTER_BIN`` (fail closed when unset or not an
existing file) EAGERLY in ``__init__`` using the provided environment
mapping exactly once and never retains that mapping afterwards, frames
one JSONL request per line with monotonic request ids, pairs responses
under a per-request timeout, and owns teardown/close/context-manager.
It knows nothing about trusted keys or tokens.

``TrustedTransport`` is the ONLY holder of the generated trusted key;
it exposes exactly the four trusted command builders and is used
exclusively by ``SyntheticEnvironmentClient``.

``BoundPlayerTransport`` holds EXACTLY ONE token and exposes only the
four typed player operations plus the package-private raw-byte submit
seam. It carries no trusted key, no generic command builder, and no
arbitrary-perspective operation, so no trusted secret exists anywhere
in the object graph reachable from a bound player client.

``argv``/``child_env`` are package-internal seams for fake-child unit
tests; production callers rely on the environment variable.
"""

from __future__ import annotations

import contextlib
import os
import queue
import secrets
import subprocess
import threading
from collections.abc import Mapping, Sequence
from pathlib import Path
from subprocess import Popen
from typing import BinaryIO, Final

from .protocol import (
    CMD_BIND_PLAYER,
    CMD_DIRECT_CALL,
    CMD_INFORMATION_STATE,
    CMD_OBSERVATION,
    CMD_RESET_SYNTHETIC,
    CMD_SHUTDOWN,
    CMD_SUBMIT,
    CMD_VISIBLE_DECISION,
    FIELD_INFORMATION_STATE_WIRE_B64,
    FIELD_OBSERVATION_WIRE_B64,
    FIELD_RESPONSE_WIRE_B64,
    FIELD_STEP_WIRE_B64,
    FIELD_VISIBLE_DECISION_WIRE_B64,
    PARAM_OP,
    PARAM_PLAYER,
    PARAM_PLAYERS,
    PARAM_ROOT_SEED_HEX,
    PARAM_TOKEN,
    PARAM_TRUSTED_KEY,
    PARSE_ERROR,
    REQUEST_TIMEOUT,
    TRANSPORT_CLOSED,
    UNKNOWN_TOKEN,
    AdapterError,
    ResponseEnvelope,
    decode_wire_payload,
    encode_request_line,
    encode_wire_payload,
    parse_response_frame,
)

BINARY_ENV_VAR: Final = "MTGML_M2_ADAPTER_BIN"
TRUSTED_KEY_ENV_VAR: Final = "MTGML_M2_ADAPTER_TRUSTED_KEY"
DEFAULT_TIMEOUT_SECONDS: Final = 60.0
TERMINATE_GRACE_SECONDS: Final = 5.0


def generate_trusted_key() -> str:
    return secrets.token_hex(32)


def build_child_environment(trusted_key: str) -> dict[str, str]:
    child = dict(os.environ)
    child[TRUSTED_KEY_ENV_VAR] = trusted_key
    return child


def resolve_binary_argv() -> tuple[str, ...]:
    raw = os.environ.get(BINARY_ENV_VAR)
    if raw is None:
        raise AdapterError(TRANSPORT_CLOSED, f"{BINARY_ENV_VAR} is not set")
    if not Path(raw).is_file():
        raise AdapterError(TRANSPORT_CLOSED, f"{BINARY_ENV_VAR} is not an existing file")
    return (raw,)


def _pump_stdout(stdout: BinaryIO, sink: queue.Queue[bytes | None]) -> None:
    try:
        while True:
            line = stdout.readline()
            if not line:
                break
            sink.put(line)
    except (OSError, ValueError):
        pass
    finally:
        sink.put(None)


def _strip_newline(raw: bytes) -> bytes:
    return raw[:-1] if raw.endswith(b"\n") else raw


class ProcessCore:
    """Blocking single-channel round-trip core to one adapter subprocess.

    Capability contract: spawn, framing, request-id pairing, timeouts,
    and teardown ONLY — no trusted key or token ever passes through this
    class, and the child environment mapping consumed at spawn time is
    deliberately not retained.

    Concurrency contract: round trips serialize behind the instance lock,
    and the lifecycle methods (``close``, ``_mark_shutdown``) take the
    same lock, so teardown never interleaves with an in-flight call. A
    lifecycle call issued while a request is outstanding waits for that
    request to finish or fail first, bounded by the per-request timeout.
    """

    def __init__(
        self,
        argv: Sequence[str],
        child_env: Mapping[str, str],
        *,
        timeout: float = DEFAULT_TIMEOUT_SECONDS,
    ) -> None:
        self._timeout = timeout
        self._argv = tuple(argv)
        self._lock = threading.Lock()
        self._responses: queue.Queue[bytes | None] = queue.Queue()
        self._next_id = 0
        self._closed = False
        # Eager fail-fast spawn: the environment mapping is consumed here,
        # exactly once, and intentionally NOT stored on the instance.
        try:
            process = Popen(
                list(self._argv),
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=None,
                env=dict(child_env),
            )
        except OSError as exc:
            raise AdapterError(TRANSPORT_CLOSED, "failed to spawn adapter binary") from exc
        stdout = process.stdout
        assert stdout is not None
        self._process: Popen[bytes] = process
        self._reader = threading.Thread(
            target=_pump_stdout, args=(stdout, self._responses), daemon=True
        )
        self._reader.start()

    def round_trip(self, command: str, params: Mapping[str, object]) -> Mapping[str, object]:
        """Send one framed request and return its successful result.

        ``ok:false`` envelopes surface verbatim as :class:`AdapterError`
        carrying the adapter's closed error code."""
        with self._lock:
            envelope = self._round_trip(command, params)
        if not envelope.ok:
            assert envelope.error_code is not None
            raise AdapterError(envelope.error_code, f"command {command!r} failed")
        assert envelope.result is not None
        return envelope.result

    def close(self) -> None:
        with self._lock:
            self._teardown(graceful=False)

    def __enter__(self) -> ProcessCore:
        return self

    def __exit__(self, *exc_info: object) -> None:
        self.close()

    def _round_trip(self, command: str, params: Mapping[str, object]) -> ResponseEnvelope:
        if self._closed:
            raise AdapterError(TRANSPORT_CLOSED, "core is closed")
        process = self._process
        self._next_id += 1
        request_id = self._next_id
        line = encode_request_line(request_id, command, params) + b"\n"
        stdin = process.stdin
        assert stdin is not None
        try:
            stdin.write(line)
            stdin.flush()
        except (OSError, ValueError) as exc:
            self._teardown(graceful=False)
            raise AdapterError(TRANSPORT_CLOSED, "failed to write request") from exc
        try:
            item = self._responses.get(timeout=self._timeout)
        except queue.Empty:
            self._teardown(graceful=False)
            raise AdapterError(REQUEST_TIMEOUT, "adapter response timed out") from None
        if item is None:
            self._teardown(graceful=False)
            raise AdapterError(TRANSPORT_CLOSED, "adapter closed its output")
        return parse_response_frame(_strip_newline(item), request_id)

    def _teardown(self, *, graceful: bool) -> None:
        """Single deterministic teardown sequence; caller holds ``_lock``.

        Order: stop the child (graceful mode signals EOF on stdin first),
        wait for it, join the reader thread, and only then close both
        pipes. The reader owns stdout while alive, so stdout is closed
        exclusively after the join; every path ends with both pipes
        closed, including timeout, crash, and shutdown.
        """
        self._closed = True
        process = self._process
        if graceful and process.poll() is None:
            stdin = process.stdin
            if stdin is not None:
                with contextlib.suppress(OSError, ValueError):
                    stdin.close()
                # Graceful mode signals EOF and gives the child time to exit
                # cleanly before any terminate is considered.
                with contextlib.suppress(subprocess.TimeoutExpired):
                    process.wait(timeout=TERMINATE_GRACE_SECONDS)
        self._stop_child(process)
        self._join_reader()
        self._close_pipes(process)

    def _stop_child(self, process: Popen[bytes]) -> None:
        if process.poll() is not None:
            return
        process.terminate()
        try:
            process.wait(timeout=TERMINATE_GRACE_SECONDS)
        except subprocess.TimeoutExpired:
            process.kill()
            with contextlib.suppress(subprocess.TimeoutExpired):
                process.wait(timeout=TERMINATE_GRACE_SECONDS)

    def _join_reader(self) -> None:
        reader = self._reader
        if reader.is_alive():
            reader.join(timeout=TERMINATE_GRACE_SECONDS)

    def _close_pipes(self, process: Popen[bytes]) -> None:
        for pipe in (process.stdin, process.stdout):
            if pipe is not None:
                with contextlib.suppress(OSError, ValueError):
                    pipe.close()

    def _mark_shutdown(self) -> None:
        with self._lock:
            self._teardown(graceful=True)


class TrustedTransport:
    """The ONLY capability holder for the generated trusted key.

    Exposes exactly the four trusted orchestration builders
    (reset_synthetic / bind_player / direct_call / shutdown) plus the
    core-close delegation. Used exclusively by
    ``SyntheticEnvironmentClient`` and never reachable from a bound
    player client."""

    def __init__(self, core: ProcessCore, trusted_key: str) -> None:
        self._core = core
        self._key = trusted_key

    @property
    def _trusted_key(self) -> str:
        return self._key

    def _invoke(self, command: str, params: Mapping[str, object]) -> Mapping[str, object]:
        """Single send primitive shared by every trusted builder; also the
        recording seam for test transports."""
        return self._core.round_trip(command, params)

    def _reset_synthetic(self, players: Sequence[str], root_seed_hex: str) -> Mapping[str, object]:
        return self._invoke(
            CMD_RESET_SYNTHETIC,
            {
                PARAM_TRUSTED_KEY: self._key,
                PARAM_PLAYERS: list(players),
                PARAM_ROOT_SEED_HEX: root_seed_hex,
            },
        )

    def _bind_player(self, player: str) -> Mapping[str, object]:
        return self._invoke(CMD_BIND_PLAYER, {PARAM_TRUSTED_KEY: self._key, PARAM_PLAYER: player})

    def _direct_call(
        self, op: str, player: str, response_wire: bytes | None
    ) -> Mapping[str, object]:
        params: dict[str, object] = {
            PARAM_TRUSTED_KEY: self._key,
            PARAM_OP: op,
            PARAM_PLAYER: player,
        }
        if response_wire is not None:
            params[FIELD_RESPONSE_WIRE_B64] = encode_wire_payload(response_wire)
        return self._invoke(CMD_DIRECT_CALL, params)

    def _shutdown(self) -> Mapping[str, object]:
        return self._invoke(CMD_SHUTDOWN, {PARAM_TRUSTED_KEY: self._key})

    def close(self) -> None:
        self._core.close()


def _result_payload(result: Mapping[str, object], field: str) -> bytes:
    """Extract and base64-decode one wire-payload field from a result."""
    if field not in result:
        raise AdapterError(PARSE_ERROR, f"result lacks {field}")
    value = result[field]
    if not isinstance(value, str):
        raise AdapterError(PARSE_ERROR, f"{field} is not a base64 string")
    return decode_wire_payload(value)


class BoundPlayerTransport:
    """Package-private seam bound to ONE token: the four typed player
    operations plus the sole raw-byte entry point beneath the client.
    Carries zero trusted commands, no generic send surface, and no key:
    no trusted secret exists anywhere in the graph reachable from here."""

    def __init__(self, core: ProcessCore, token: str) -> None:
        self._core = core
        self._token = token

    def observation(self) -> bytes:
        result = self._core.round_trip(CMD_OBSERVATION, {PARAM_TOKEN: self._token})
        return _result_payload(result, FIELD_OBSERVATION_WIRE_B64)

    def information_state(self) -> bytes:
        result = self._core.round_trip(CMD_INFORMATION_STATE, {PARAM_TOKEN: self._token})
        return _result_payload(result, FIELD_INFORMATION_STATE_WIRE_B64)

    def visible_decision(self) -> bytes | None:
        result = self._core.round_trip(CMD_VISIBLE_DECISION, {PARAM_TOKEN: self._token})
        if FIELD_VISIBLE_DECISION_WIRE_B64 not in result:
            raise AdapterError(PARSE_ERROR, "visible_decision result lacks its field")
        value = result[FIELD_VISIBLE_DECISION_WIRE_B64]
        if value is None:
            return None
        if not isinstance(value, str):
            raise AdapterError(PARSE_ERROR, "visible_decision payload is not a string")
        return decode_wire_payload(value)

    def submit(self, response_wire: bytes) -> bytes:
        return self._submit_wire_bytes(self._token, response_wire)

    def _submit_wire_bytes(self, token: str, raw_bytes: bytes) -> bytes:
        if token != self._token:
            raise AdapterError(UNKNOWN_TOKEN, "token does not match this binding")
        result = self._core.round_trip(
            CMD_SUBMIT,
            {
                PARAM_TOKEN: self._token,
                FIELD_RESPONSE_WIRE_B64: encode_wire_payload(raw_bytes),
            },
        )
        value = result.get(FIELD_STEP_WIRE_B64)
        if not isinstance(value, str):
            raise AdapterError(PARSE_ERROR, "submit result lacks its step field")
        return decode_wire_payload(value)
