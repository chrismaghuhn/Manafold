"""JSONL envelope vocabulary for the temporary M2 semantic adapter.

Mirrors ``tools/m2-semantic-adapter/src/protocol.rs``: protocol version,
the closed adapter error-code set, request/response builders and parsers,
and base64 confinement helpers for ``*_wire_b64`` payload fields.

``TRANSPORT_CLOSED`` and ``REQUEST_TIMEOUT`` are LOCAL transport-layer
codes only; they never appear on the wire and are deliberately outside
the closed adapter-code set.
"""

from __future__ import annotations

import base64
import json
from collections.abc import Mapping
from dataclasses import dataclass
from typing import Final, NoReturn

PROTOCOL_VERSION: Final = 1

PARSE_ERROR: Final = "parse_error"
UNKNOWN_COMMAND: Final = "unknown_command"
INVALID_PARAMS: Final = "invalid_params"
UNKNOWN_TOKEN: Final = "unknown_token"
OVERSIZED_INPUT: Final = "oversized_input"
INTERNAL_ERROR: Final = "internal_error"
MALFORMED_RESPONSE: Final = "malformed_response"
SERVICE_UNAVAILABLE: Final = "service_unavailable"

ADAPTER_CODES: Final[frozenset[str]] = frozenset(
    {
        PARSE_ERROR,
        UNKNOWN_COMMAND,
        INVALID_PARAMS,
        UNKNOWN_TOKEN,
        OVERSIZED_INPUT,
        INTERNAL_ERROR,
        MALFORMED_RESPONSE,
        SERVICE_UNAVAILABLE,
    }
)

TRANSPORT_CLOSED: Final = "transport_closed"
REQUEST_TIMEOUT: Final = "request_timeout"

CMD_RESET_SYNTHETIC: Final = "reset_synthetic"
CMD_BIND_PLAYER: Final = "bind_player"
CMD_DIRECT_CALL: Final = "direct_call"
CMD_SHUTDOWN: Final = "shutdown"
CMD_OBSERVATION: Final = "observation"
CMD_INFORMATION_STATE: Final = "information_state"
CMD_VISIBLE_DECISION: Final = "visible_decision"
CMD_SUBMIT: Final = "submit"

TRUSTED_COMMANDS: Final[tuple[str, ...]] = (
    CMD_RESET_SYNTHETIC,
    CMD_BIND_PLAYER,
    CMD_DIRECT_CALL,
    CMD_SHUTDOWN,
)
PLAYER_COMMANDS: Final[tuple[str, ...]] = (
    CMD_OBSERVATION,
    CMD_INFORMATION_STATE,
    CMD_VISIBLE_DECISION,
    CMD_SUBMIT,
)

PARAM_TRUSTED_KEY: Final = "trusted_key"
PARAM_TOKEN: Final = "token"
PARAM_PLAYER: Final = "player"
PARAM_PLAYERS: Final = "players"
PARAM_ROOT_SEED_HEX: Final = "root_seed_hex"
RESULT_TOKEN: Final = "token"

FIELD_OBSERVATION_WIRE_B64: Final = "observation_wire_b64"
FIELD_INFORMATION_STATE_WIRE_B64: Final = "information_state_wire_b64"
FIELD_VISIBLE_DECISION_WIRE_B64: Final = "visible_decision_wire_b64"
FIELD_STEP_WIRE_B64: Final = "step_wire_b64"
FIELD_RESPONSE_WIRE_B64: Final = "response_wire_b64"


class AdapterError(ValueError):
    """Local failure carrying a closed adapter or local transport code."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message


@dataclass(frozen=True, slots=True)
class ResponseEnvelope:
    """Parsed response frame correlated by its echoed request id."""

    id: int
    ok: bool
    result: Mapping[str, object] | None
    error_code: str | None


def encode_request_line(request_id: int, command: str, params: Mapping[str, object]) -> bytes:
    request: dict[str, object] = {
        "v": PROTOCOL_VERSION,
        "id": request_id,
        "cmd": command,
        "params": dict(params),
    }
    return json.dumps(request, separators=(",", ":")).encode("utf-8")


def _reject(message: str) -> NoReturn:
    raise AdapterError(PARSE_ERROR, message)


def parse_response_frame(raw: bytes, expected_id: int) -> ResponseEnvelope:
    try:
        value = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError):
        _reject("frame is not UTF-8 JSON")
    if not isinstance(value, dict):
        _reject("frame is not a JSON object")
    version = value.get("v")
    if isinstance(version, bool) or not isinstance(version, int) or version != PROTOCOL_VERSION:
        _reject("unsupported envelope version")
    identifier = value.get("id")
    if isinstance(identifier, bool) or not isinstance(identifier, int):
        _reject("request id echo is not an integer")
    if identifier != expected_id:
        _reject("request id echo mismatch")
    ok = value.get("ok")
    if not isinstance(ok, bool):
        _reject("ok flag is not boolean")
    if ok:
        result = value.get("result")
        if not isinstance(result, dict):
            _reject("ok frame lacks a result object")
        return ResponseEnvelope(identifier, True, result, None)
    error = value.get("error")
    if not isinstance(error, dict):
        _reject("error frame lacks an error object")
    code = error.get("code")
    if not isinstance(code, str):
        _reject("error code is not a string")
    if code not in ADAPTER_CODES:
        _reject("error code is outside the closed adapter-code set")
    return ResponseEnvelope(identifier, False, None, code)


def encode_wire_payload(payload: bytes) -> str:
    return base64.b64encode(payload).decode("ascii")


def decode_wire_payload(value: object) -> bytes:
    if not isinstance(value, str):
        _reject("wire payload is not a base64 string")
    try:
        return base64.b64decode(value.encode("ascii"), validate=True)
    except (ValueError, UnicodeEncodeError) as exc:
        raise AdapterError(PARSE_ERROR, "wire payload is not valid base64") from exc
