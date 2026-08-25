"""Typed clients over the M2 semantic adapter transport.

``SyntheticEnvironmentClient`` is the trusted orchestration surface
(reset/bind/shutdown): it generates the trusted key once, builds the
child environment copy, eagerly assembles the ``ProcessCore`` plus
``TrustedTransport`` pair, and keeps the key ONLY inside the trusted
transport; it never exposes it. ``AdapterPlayerClient`` implements the
``mtgml.PlayerClient`` protocol exactly, holds exactly one token inside
its ``BoundPlayerTransport``, decodes payloads through the real mtgml
codecs, and carries no generic command method and no choice-making
logic; no trusted secret exists anywhere in its reachable object graph.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
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

from . import process as _adapter_process
from .process import (
    DEFAULT_TIMEOUT_SECONDS,
    BoundPlayerTransport,
    ProcessCore,
    TrustedTransport,
)
from .protocol import PARSE_ERROR, RESULT_TOKEN, SERVICE_UNAVAILABLE, AdapterError
from .submission import encode_decision_response_submission_v2

_DEFAULT_PLAYERS: tuple[str, str] = ("1", "2")


def _decode_contract(contract: str, payload: bytes) -> object:
    """Decode one INBOUND adapter output payload through the real codec.

    Rust->Python output corruption is an adapter health failure, NOT a
    player-submission boundary error: layer-A ``malformed_response`` is
    normatively reserved for PLAYER-SUBMITTED bad bytes judged by the
    adapter itself (those arrive verbatim as ok:false envelope codes), so
    local failures decoding successful outputs fail closed as
    ``service_unavailable`` instead.
    """
    try:
        return decode_canonical(contract, payload)
    except ValueError as exc:
        raise AdapterError(
            SERVICE_UNAVAILABLE, f"adapter output rejected by the {contract} codec"
        ) from exc


class SyntheticEnvironmentClient:
    def __init__(
        self,
        *,
        timeout: float = DEFAULT_TIMEOUT_SECONDS,
        argv: Sequence[str] | None = None,
    ) -> None:
        key = _adapter_process.generate_trusted_key()
        child_env = _adapter_process.build_child_environment(key)
        core_argv = _adapter_process.resolve_binary_argv() if argv is None else tuple(argv)
        self._core: ProcessCore = self._spawn_core(core_argv, child_env, timeout)
        self._transport = TrustedTransport(self._core, key)
        self._shutdown_done = False

    def _spawn_core(
        self, argv: tuple[str, ...], child_env: dict[str, str], timeout: float
    ) -> ProcessCore:
        """Package-private assembly seam: production always builds a plain
        ``ProcessCore`` here; test subclasses may swap in an instrumented
        one. The environment mapping is consumed by the eager spawn and
        never retained."""
        return ProcessCore(argv, child_env, timeout=timeout)

    def reset_synthetic(
        self, players: tuple[str, str] = _DEFAULT_PLAYERS, *, root_seed_hex: str
    ) -> None:
        self._transport._reset_synthetic(players, root_seed_hex)

    def bind_player(self, player: str) -> AdapterPlayerClient:
        result = self._transport._bind_player(player)
        token = result.get(RESULT_TOKEN)
        if not isinstance(token, str) or not token:
            raise AdapterError(PARSE_ERROR, "bind_player result lacks a token")
        return AdapterPlayerClient(BoundPlayerTransport(self._core, token), token)

    def shutdown(self) -> None:
        """Idempotent, with fixed ordering: request the wire shutdown and
        drain its reply through the trusted transport first, record
        completion, then release the subprocess via a graceful core
        teardown."""
        if self._shutdown_done:
            return
        self._transport._shutdown()
        self._shutdown_done = True
        self._core._mark_shutdown()

    def __enter__(self) -> Self:
        return self

    def __exit__(self, *exc_info: object) -> None:
        self.shutdown()

    def _trusted_direct_call(
        self, op: str, player: str, response_wire: bytes | None
    ) -> Mapping[str, object]:
        """Package-private passthrough to the trusted ``direct_call`` route.

        Test-harness seam only: executes one player operation against
        ``player`` without holding a token, carrying an already-encoded
        decision-response payload when one is required. It adds no choosing
        logic and stays invisible to the public API inventory.
        """
        return self._transport._direct_call(op, player, response_wire)


class AdapterPlayerClient:
    def __init__(self, transport: BoundPlayerTransport, token: str) -> None:
        self._transport = transport
        self._token = token

    def observation(self) -> ObservationEnvelope:
        payload = self._transport.observation()
        return cast(ObservationEnvelope, _decode_contract(OBSERVATION_SCHEMA, payload))

    def information_state(self) -> PlayerInformationStateV2:
        payload = self._transport.information_state()
        return cast(
            PlayerInformationStateV2,
            _decode_contract(INFORMATION_STATE_SCHEMA_V2, payload),
        )

    def visible_decision(self) -> PlayerDecisionRequestV2 | None:
        payload = self._transport.visible_decision()
        if payload is None:
            return None
        return cast(
            PlayerDecisionRequestV2,
            _decode_contract(PLAYER_DECISION_REQUEST_V2_SCHEMA, payload),
        )

    def submit(self, response: DecisionResponseV2) -> PlayerStepV2:
        raw = encode_decision_response_submission_v2(response)
        payload = self._transport.submit(raw)
        return cast(PlayerStepV2, _decode_contract(PLAYER_STEP_SCHEMA_V2, payload))


if TYPE_CHECKING:

    def _protocol_witness(client: AdapterPlayerClient) -> PlayerClient:
        return client
