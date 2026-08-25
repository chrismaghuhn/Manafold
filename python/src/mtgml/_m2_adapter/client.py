"""Typed clients over the M2 semantic adapter transport.

``SyntheticEnvironmentClient`` is the trusted orchestration surface
(reset/bind/shutdown); it keeps the trusted key internally and never
exposes it. ``AdapterPlayerClient`` implements the ``mtgml.PlayerClient``
protocol exactly, holds exactly one token, decodes payloads through the
real mtgml codecs, and carries no generic command method and no
choice-making logic.
"""

from __future__ import annotations

from collections.abc import Callable, Mapping
from typing import TYPE_CHECKING, Self, cast

from ..decision import (
    PLAYER_DECISION_REQUEST_V2_SCHEMA,
    DecisionResponseV2,
    PlayerDecisionRequestV2,
)
from ..observation import (
    INFORMATION_STATE_SCHEMA_V2,
    OBSERVATION_SCHEMA,
    PLAYER_STEP_SCHEMA_V2,
    ObservationEnvelope,
    PlayerInformationStateV2,
    PlayerStepV2,
)
from ..wire import decode_canonical

if TYPE_CHECKING:
    from ..player_client import PlayerClient

from .process import SubprocessTransport
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
    MALFORMED_RESPONSE,
    PARAM_OP,
    PARAM_PLAYER,
    PARAM_PLAYERS,
    PARAM_ROOT_SEED_HEX,
    PARAM_TOKEN,
    PARAM_TRUSTED_KEY,
    PARSE_ERROR,
    RESULT_TOKEN,
    AdapterError,
    decode_wire_payload,
    encode_wire_payload,
)
from .submission import encode_decision_response_submission_v2

_DEFAULT_PLAYERS: tuple[str, str] = ("1", "2")


def _decode_contract(contract: str, payload: bytes) -> object:
    try:
        return decode_canonical(contract, payload)
    except ValueError as exc:
        raise AdapterError(
            MALFORMED_RESPONSE, f"adapter payload rejected by the {contract} codec"
        ) from exc


class SyntheticEnvironmentClient:
    def __init__(
        self,
        transport: SubprocessTransport | Callable[[], SubprocessTransport],
    ) -> None:
        if isinstance(transport, SubprocessTransport) or not callable(transport):
            self._transport = transport
        else:
            self._transport = transport()
        self._shutdown_done = False

    def reset_synthetic(
        self, players: tuple[str, str] = _DEFAULT_PLAYERS, *, root_seed_hex: str
    ) -> None:
        self._trusted_call(
            CMD_RESET_SYNTHETIC,
            {
                PARAM_TRUSTED_KEY: self._transport._trusted_key,
                PARAM_PLAYERS: list(players),
                PARAM_ROOT_SEED_HEX: root_seed_hex,
            },
        )

    def bind_player(self, player: str) -> AdapterPlayerClient:
        result = self._trusted_call(
            CMD_BIND_PLAYER,
            {PARAM_TRUSTED_KEY: self._transport._trusted_key, PARAM_PLAYER: player},
        )
        token = result.get(RESULT_TOKEN)
        if not isinstance(token, str) or not token:
            raise AdapterError(PARSE_ERROR, "bind_player result lacks a token")
        return AdapterPlayerClient(self._transport, token)

    def shutdown(self) -> None:
        """Idempotent, with fixed ordering: request the wire shutdown and
        drain its reply through the transport first, record completion,
        then release the subprocess via a graceful transport teardown."""
        if self._shutdown_done:
            return
        self._trusted_call(CMD_SHUTDOWN, {PARAM_TRUSTED_KEY: self._transport._trusted_key})
        self._shutdown_done = True
        self._transport._mark_shutdown()

    def __enter__(self) -> Self:
        return self

    def __exit__(self, *exc_info: object) -> None:
        self.shutdown()

    def _trusted_call(self, command: str, params: dict[str, object]) -> Mapping[str, object]:
        return self._transport.call(command, params)

    def _trusted_direct_call(
        self, op: str, player: str, response_wire: bytes | None
    ) -> Mapping[str, object]:
        """Package-private passthrough to the trusted ``direct_call`` route.

        Test-harness seam only: executes one player operation against
        ``player`` without holding a token, carrying an already-encoded
        decision-response payload when one is required. It adds no choosing
        logic and stays invisible to the public API inventory.
        """
        params: dict[str, object] = {
            PARAM_TRUSTED_KEY: self._transport._trusted_key,
            PARAM_OP: op,
            PARAM_PLAYER: player,
        }
        if response_wire is not None:
            params[FIELD_RESPONSE_WIRE_B64] = encode_wire_payload(response_wire)
        return self._trusted_call(CMD_DIRECT_CALL, params)


class AdapterPlayerClient:
    def __init__(self, transport: SubprocessTransport, token: str) -> None:
        self._transport = transport
        self._token = token

    def observation(self) -> ObservationEnvelope:
        result = self._transport.call(CMD_OBSERVATION, {PARAM_TOKEN: self._token})
        payload = _wire_payload(result, FIELD_OBSERVATION_WIRE_B64)
        return cast(ObservationEnvelope, _decode_contract(OBSERVATION_SCHEMA, payload))

    def information_state(self) -> PlayerInformationStateV2:
        result = self._transport.call(CMD_INFORMATION_STATE, {PARAM_TOKEN: self._token})
        payload = _wire_payload(result, FIELD_INFORMATION_STATE_WIRE_B64)
        return cast(
            PlayerInformationStateV2,
            _decode_contract(INFORMATION_STATE_SCHEMA_V2, payload),
        )

    def visible_decision(self) -> PlayerDecisionRequestV2 | None:
        result = self._transport.call(CMD_VISIBLE_DECISION, {PARAM_TOKEN: self._token})
        if FIELD_VISIBLE_DECISION_WIRE_B64 not in result:
            raise AdapterError(PARSE_ERROR, "visible_decision result lacks its field")
        value = result[FIELD_VISIBLE_DECISION_WIRE_B64]
        if value is None:
            return None
        if not isinstance(value, str):
            raise AdapterError(PARSE_ERROR, "visible_decision payload is not a string")
        payload = decode_wire_payload(value)
        return cast(
            PlayerDecisionRequestV2,
            _decode_contract(PLAYER_DECISION_REQUEST_V2_SCHEMA, payload),
        )

    def submit(self, response: DecisionResponseV2) -> PlayerStepV2:
        raw = encode_decision_response_submission_v2(response)
        result = self._transport.call(
            CMD_SUBMIT,
            {
                PARAM_TOKEN: self._token,
                FIELD_RESPONSE_WIRE_B64: encode_wire_payload(raw),
            },
        )
        payload = _wire_payload(result, FIELD_STEP_WIRE_B64)
        return cast(PlayerStepV2, _decode_contract(PLAYER_STEP_SCHEMA_V2, payload))


def _wire_payload(result: Mapping[str, object], field: str) -> bytes:
    if field not in result:
        raise AdapterError(PARSE_ERROR, f"result lacks {field}")
    value = result[field]
    if not isinstance(value, str):
        raise AdapterError(PARSE_ERROR, f"{field} is not a base64 string")
    return decode_wire_payload(value)


if TYPE_CHECKING:

    def _protocol_witness(client: AdapterPlayerClient) -> PlayerClient:
        return client
