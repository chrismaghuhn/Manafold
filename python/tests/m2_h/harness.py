"""Shared harness for the H.4-i M2.H lockstep twin scenarios.

Deliberately NOT ``test_``-prefixed, so no runner collects it. This module
only provides:

- ``build_twin_clients``: a context-managed pair of
  ``SyntheticEnvironmentClient`` instances built on identical trusted
  reset inputs (default players ``("1", "2")`` plus one seed), whose
  teardown is exception-safe: both twins always attempt their full
  teardown and a scenario error is never masked by a teardown failure;
- payload-only comparison helpers that never surface envelope ids or
  tokens;
- explicit answer constructors that read ONLY public request data.

Every scenario drives the REAL adapter binary resolved through
``MTGML_M2_ADAPTER_BIN``; nothing here fabricates responses.
"""

from __future__ import annotations

import contextlib
import inspect
import sys
import unittest
from collections.abc import Callable, Iterator, Mapping
from pathlib import Path
from typing import Any, Final, NamedTuple, cast

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "python" / "src"))

from mtgml._m2_adapter import AdapterPlayerClient, SyntheticEnvironmentClient
from mtgml._m2_adapter.process import SubprocessTransport
from mtgml._m2_adapter.protocol import decode_wire_payload
from mtgml._m2_adapter.submission import encode_decision_response_submission_v2
from mtgml.canonical import canonical_json_bytes
from mtgml.decision import (
    DECISION_RESPONSE_V2_SCHEMA,
    PLAYER_DECISION_REQUEST_V2_SCHEMA,
    DecisionAnswerV2,
    DecisionResponseV2,
    PlayerDecisionRequestV2,
)
from mtgml.observation import (
    INFORMATION_STATE_SCHEMA_V2,
    PLAYER_STEP_SCHEMA_V2,
    PlayerInformationStateV2,
    PlayerStepV2,
)
from mtgml.wire import decode_canonical

PLAYER_CLIENT_PUBLIC_METHODS: Final[frozenset[str]] = frozenset(
    {"observation", "information_state", "visible_decision", "submit"}
)
SYNTHETIC_PUBLIC_METHODS: Final[frozenset[str]] = frozenset(
    {"reset_synthetic", "bind_player", "shutdown"}
)
TRUSTED_DIRECT_CALL_NAME: Final = "_trusted_direct_call"

EXPECTED_DECISION_KIND_SEQUENCE: Final[tuple[str, ...]] = (
    "choose_one",
    "choose_number",
    "choose_many",
    "order",
)

SEED_ABSENCE_PROBE_KEY: Final = "root_seed"

_PAYLOAD_SUFFIX: Final = "_wire_b64"


class RecordingTransport(SubprocessTransport):
    """Transport that records the result mapping of every successful round trip."""

    def __init__(self) -> None:
        super().__init__()
        self.recorded_results: list[dict[str, object]] = []

    def call(self, command: str, params: Mapping[str, object]) -> Mapping[str, object]:
        result = super().call(command, params)
        self.recorded_results.append(dict(result))
        return result


class TwinClients(NamedTuple):
    env_a: SyntheticEnvironmentClient
    client_a1: AdapterPlayerClient
    client_a2: AdapterPlayerClient
    env_b: SyntheticEnvironmentClient
    client_b1: AdapterPlayerClient
    client_b2: AdapterPlayerClient


def _guarded_teardown_step(
    label: str, action: Callable[[], object], errors: list[BaseException]
) -> None:
    """Run one teardown step; record failures so later steps still run."""
    try:
        action()
    except Exception as exc:
        print(f"[m2_h] twin teardown error ({label}): {exc!r}", file=sys.stderr)
        errors.append(exc)


@contextlib.contextmanager
def build_twin_clients(seed_hex: str) -> Iterator[TwinClients]:
    """Two identical-reset synthetic environments, each with P1+P2 bound.

    Teardown guarantee: BOTH twins always attempt their full teardown
    sequence (environment shutdown, then transport close) even when the
    scenario body or the first teardown step raised. The original error
    is never masked: when the body raised, every teardown failure stays
    suppressed with a stderr log line and the body error surfaces
    unchanged. When the body succeeded, the first real teardown error
    surfaces and any later one remains suppressed-with-logging.
    """
    transport_a: RecordingTransport | None = None
    transport_b: RecordingTransport | None = None
    env_a: SyntheticEnvironmentClient | None = None
    env_b: SyntheticEnvironmentClient | None = None
    body_error: BaseException | None = None
    try:
        transport_a = RecordingTransport()
        env_a = SyntheticEnvironmentClient(transport_a)
        transport_b = RecordingTransport()
        env_b = SyntheticEnvironmentClient(transport_b)
        env_a.reset_synthetic(root_seed_hex=seed_hex)
        env_b.reset_synthetic(root_seed_hex=seed_hex)
        yield TwinClients(
            env_a=env_a,
            client_a1=env_a.bind_player("1"),
            client_a2=env_a.bind_player("2"),
            env_b=env_b,
            client_b1=env_b.bind_player("1"),
            client_b2=env_b.bind_player("2"),
        )
    except BaseException as exc:
        body_error = exc
        raise
    finally:
        errors: list[BaseException] = []
        for label, environment, transport in (
            ("A", env_a, transport_a),
            ("B", env_b, transport_b),
        ):
            if environment is not None:
                _guarded_teardown_step(
                    f"twin {label} environment shutdown", environment.shutdown, errors
                )
            if transport is not None:
                _guarded_teardown_step(f"twin {label} transport close", transport.close, errors)
        if errors and body_error is None:
            raise errors[0]
        if errors:
            print(
                f"[m2_h] {len(errors)} twin teardown error(s) suppressed "
                "behind the original scenario error",
                file=sys.stderr,
            )


def recorded_results(environment: SyntheticEnvironmentClient) -> list[dict[str, object]]:
    transport = environment._transport
    assert isinstance(transport, RecordingTransport), (
        "twin environments must be built on RecordingTransports"
    )
    return transport.recorded_results


def payload_only(result: Mapping[str, object]) -> dict[str, object]:
    """Result view restricted to wire-payload fields; envelope routing
    material (ids, tokens) can never survive this projection."""
    return {key: value for key, value in result.items() if key.endswith(_PAYLOAD_SUFFIX)}


def assert_payload_equality(
    test: unittest.TestCase,
    left: Mapping[str, object],
    right: Mapping[str, object],
    context: str,
) -> None:
    test.assertEqual(payload_only(left), payload_only(right), context)


def wire_payload_bytes(value: Any) -> bytes:
    """Canonical wire bytes of a decoded contract object via its own
    deterministic re-encode; equal bytes imply equal payloads."""
    return canonical_json_bytes(value.to_wire())


def optional_wire_payload_bytes(value: Any) -> bytes | None:
    return None if value is None else wire_payload_bytes(value)


def select_one_first_offered(request: PlayerDecisionRequestV2) -> DecisionAnswerV2:
    return DecisionAnswerV2("select_one", candidate_id=request.candidates[0].candidate_id)


def choose_number_spec_minimum(request: PlayerDecisionRequestV2) -> DecisionAnswerV2:
    minimum = request.decision.minimum
    if minimum is None:
        raise AssertionError("choose_number request lacks a minimum bound")
    return DecisionAnswerV2("choose_number", value=minimum)


def select_many_first_minimum_offered(request: PlayerDecisionRequestV2) -> DecisionAnswerV2:
    minimum = request.decision.minimum
    if minimum is None:
        raise AssertionError("choose_many request lacks a minimum cardinality")
    ascending = sorted(candidate.candidate_id for candidate in request.candidates)
    return DecisionAnswerV2("select_many", candidate_ids=tuple(ascending[:minimum]))


def order_reversed_offered(request: PlayerDecisionRequestV2) -> DecisionAnswerV2:
    offered = [candidate.candidate_id for candidate in request.candidates]
    return DecisionAnswerV2("order", candidate_ids=tuple(reversed(offered)))


ANSWER_CONSTRUCTORS: Final[dict[str, Callable[[PlayerDecisionRequestV2], DecisionAnswerV2]]] = {
    "choose_one": select_one_first_offered,
    "choose_number": choose_number_spec_minimum,
    "choose_many": select_many_first_minimum_offered,
    "order": order_reversed_offered,
}


def response_for(request: PlayerDecisionRequestV2, answer: DecisionAnswerV2) -> DecisionResponseV2:
    return DecisionResponseV2(
        DECISION_RESPONSE_V2_SCHEMA,
        request.player_decision_id,
        request.state_revision,
        answer,
    )


def submit_response_bytes(response: DecisionResponseV2) -> bytes:
    return encode_decision_response_submission_v2(response)


def direct_call_result(
    environment: SyntheticEnvironmentClient,
    op: str,
    player: str,
    response_wire: bytes | None = None,
) -> Mapping[str, object]:
    return environment._trusted_direct_call(op, player, response_wire)


def payload_field(result: Mapping[str, object], field: str) -> bytes | None:
    value = result.get(field)
    return None if value is None else decode_wire_payload(value)


def decode_request(raw: bytes) -> PlayerDecisionRequestV2:
    return cast(
        PlayerDecisionRequestV2,
        decode_canonical(PLAYER_DECISION_REQUEST_V2_SCHEMA, raw),
    )


def decode_information_state(raw: bytes) -> PlayerInformationStateV2:
    return cast(
        PlayerInformationStateV2,
        decode_canonical(INFORMATION_STATE_SCHEMA_V2, raw),
    )


def decode_step(raw: bytes) -> PlayerStepV2:
    return cast(PlayerStepV2, decode_canonical(PLAYER_STEP_SCHEMA_V2, raw))


def public_method_names(instance: object) -> set[str]:
    return {
        name
        for name, value in inspect.getmembers(instance, inspect.ismethod)
        if not name.startswith("_")
    }


def assert_public_surface_unchanged(
    test: unittest.TestCase,
    player: AdapterPlayerClient,
    environment: SyntheticEnvironmentClient,
) -> None:
    test.assertEqual(public_method_names(player), set(PLAYER_CLIENT_PUBLIC_METHODS))
    test.assertEqual(public_method_names(environment), set(SYNTHETIC_PUBLIC_METHODS))
    test.assertIn(TRUSTED_DIRECT_CALL_NAME, dir(environment))
