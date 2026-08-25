"""Subprocess transport for the M2 semantic adapter.

Spawns the adapter binary located via ``MTGML_M2_ADAPTER_BIN`` (fail
closed when unset or not an existing file), frames one JSONL request per
line with monotonic request ids, and pairs responses under a per-request
timeout. The generated trusted key exists only in the child's copied
environment mapping; the parent ``os.environ`` is never mutated.

``argv`` is a package-internal seam for fake-child unit tests; production
callers rely on the environment variable.
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
    CMD_SUBMIT,
    FIELD_RESPONSE_WIRE_B64,
    FIELD_STEP_WIRE_B64,
    PARAM_TOKEN,
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
    finally:
        sink.put(None)


class SubprocessTransport:
    def __init__(
        self,
        *,
        timeout: float = DEFAULT_TIMEOUT_SECONDS,
        argv: Sequence[str] | None = None,
    ) -> None:
        self._timeout = timeout
        self._argv = tuple(argv) if argv is not None else resolve_binary_argv()
        self._trusted_key_value = generate_trusted_key()
        self._child_env = build_child_environment(self._trusted_key_value)
        self._lock = threading.Lock()
        self._process: Popen[bytes] | None = None
        self._reader: threading.Thread | None = None
        self._responses: queue.Queue[bytes | None] = queue.Queue()
        self._next_id = 0
        self._closed = False

    @property
    def _trusted_key(self) -> str:
        return self._trusted_key_value

    @property
    def _child_environment(self) -> Mapping[str, str]:
        return dict(self._child_env)

    def call(self, command: str, params: Mapping[str, object]) -> Mapping[str, object]:
        with self._lock:
            envelope = self._round_trip(command, params)
        if not envelope.ok:
            assert envelope.error_code is not None
            raise AdapterError(envelope.error_code, f"command {command!r} failed")
        assert envelope.result is not None
        return envelope.result

    def close(self) -> None:
        self._terminate()
        reader = self._reader
        if reader is not None and reader.is_alive():
            reader.join(timeout=TERMINATE_GRACE_SECONDS)

    def __enter__(self) -> SubprocessTransport:
        return self

    def __exit__(self, *exc_info: object) -> None:
        self.close()

    def _round_trip(self, command: str, params: Mapping[str, object]) -> ResponseEnvelope:
        if self._closed:
            raise AdapterError(TRANSPORT_CLOSED, "transport is closed")
        process = self._ensure_spawned()
        self._next_id += 1
        request_id = self._next_id
        line = encode_request_line(request_id, command, params) + b"\n"
        stdin = process.stdin
        assert stdin is not None
        try:
            stdin.write(line)
            stdin.flush()
        except (OSError, ValueError) as exc:
            self._terminate()
            raise AdapterError(TRANSPORT_CLOSED, "failed to write request") from exc
        try:
            item = self._responses.get(timeout=self._timeout)
        except queue.Empty:
            self._terminate()
            raise AdapterError(REQUEST_TIMEOUT, "adapter response timed out") from None
        if item is None:
            self._terminate()
            raise AdapterError(TRANSPORT_CLOSED, "adapter closed its output")
        return parse_response_frame(_strip_newline(item), request_id)

    def _ensure_spawned(self) -> Popen[bytes]:
        if self._process is not None:
            return self._process
        try:
            process = Popen(
                list(self._argv),
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=None,
                env=self._child_env,
            )
        except OSError as exc:
            raise AdapterError(TRANSPORT_CLOSED, "failed to spawn adapter binary") from exc
        stdout = process.stdout
        assert stdout is not None
        self._process = process
        self._reader = threading.Thread(
            target=_pump_stdout, args=(stdout, self._responses), daemon=True
        )
        self._reader.start()
        return process

    def _terminate(self) -> None:
        self._closed = True
        process = self._process
        if process is None or process.poll() is not None:
            return
        process.terminate()
        try:
            process.wait(timeout=TERMINATE_GRACE_SECONDS)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=TERMINATE_GRACE_SECONDS)

    def _mark_shutdown(self) -> None:
        self._closed = True
        process = self._process
        if process is None:
            return
        stdin = process.stdin
        if stdin is not None:
            with contextlib.suppress(OSError):
                stdin.close()
        try:
            process.wait(timeout=TERMINATE_GRACE_SECONDS)
        except subprocess.TimeoutExpired:
            self._terminate()


def _strip_newline(raw: bytes) -> bytes:
    return raw[:-1] if raw.endswith(b"\n") else raw


class RestrictedPlayerTransport:
    """Package-private seam bound to ONE token and the single player
    submit operation: the sole raw-byte entry point beneath the client.
    Carries zero trusted commands and no generic send surface."""

    def __init__(self, transport: SubprocessTransport, token: str) -> None:
        self._transport = transport
        self._token = token

    def _submit_wire_bytes(self, token: str, raw_bytes: bytes) -> bytes:
        if token != self._token:
            raise AdapterError(UNKNOWN_TOKEN, "token does not match this binding")
        result = self._transport.call(
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
