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
from mtgml._m2_adapter.process import ProcessCore
from mtgml._m2_adapter.protocol import (
    CMD_INFORMATION_STATE,
    CMD_SUBMIT,
    CMD_VISIBLE_DECISION,
    FIELD_INFORMATION_STATE_WIRE_B64,
    FIELD_STEP_WIRE_B64,
    FIELD_VISIBLE_DECISION_WIRE_B64,
    decode_wire_payload,
)
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

# S2 binding-permanence inventory: the ONLY attribute name on a bound player
# client that may even mention binding/token/perspective is the single token
# slot itself; the ONLY submit-bearing names below the client are the four-op
# transport's public ``submit`` plus its package-private raw-byte seam.
BINDING_SURFACE_MARKERS: Final[tuple[str, ...]] = ("bind", "token", "perspective")
PLAYER_CLIENT_BINDING_INVENTORY: Final[frozenset[str]] = frozenset({"_token"})
PLAYER_CLIENT_SLOT_INVENTORY: Final[frozenset[str]] = frozenset({"_transport", "_token"})
RESTRICTED_SEAM_SUBMIT_SURFACE: Final[frozenset[str]] = frozenset({"submit", "_submit_wire_bytes"})

EXPECTED_DECISION_KIND_SEQUENCE: Final[tuple[str, ...]] = (
    "choose_one",
    "choose_number",
    "choose_many",
    "order",
)

SEED_ABSENCE_PROBE_KEY: Final = "root_seed"

PLAYER_ONE: Final = "1"
PLAYER_TWO: Final = "2"

_PAYLOAD_SUFFIX: Final = "_wire_b64"


class RecordingCore(ProcessCore):
    """Core that records the result mapping of every successful round trip —
    trusted commands AND token-scoped player commands alike, since both
    share this single child channel."""

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        super().__init__(*args, **kwargs)
        self.recorded_results: list[dict[str, object]] = []

    def round_trip(self, command: str, params: Mapping[str, object]) -> Mapping[str, object]:
        result = super().round_trip(command, params)
        self.recorded_results.append(dict(result))
        return result


class RecordingEnvironment(SyntheticEnvironmentClient):
    """Production-assembled environment (eager core spawn + trusted
    transport) whose shared core records every successful round trip for
    audit windows."""

    def _spawn_core(
        self, argv: tuple[str, ...], child_env: dict[str, str], timeout: float
    ) -> ProcessCore:
        return RecordingCore(argv, child_env, timeout=timeout)


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
def build_twin_clients(seed_hex_a: str, seed_hex_b: str | None = None) -> Iterator[TwinClients]:
    """Two identically-reset synthetic environments, each with P1+P2 bound.

    ``seed_hex_b`` defaults to ``seed_hex_a``, yielding true twins on
    identical trusted reset inputs (S1/S4/S5). Passing a DIFFERENT second
    seed yields the S8 paired-hidden-variant pair: two independent trusted
    seeds behind identical players and reset shapes.

    Teardown guarantee: BOTH twins always attempt their full teardown
    sequence (environment shutdown, then core close) even when the
    scenario body or the first teardown step raised. The original error
    is never masked: when the body raised, every teardown failure stays
    suppressed with a stderr log line and the body error surfaces
    unchanged. When the body succeeded, the first real teardown error
    surfaces and any later one remains suppressed-with-logging.
    """
    environment_a: RecordingEnvironment | None = None
    environment_b: RecordingEnvironment | None = None
    body_error: BaseException | None = None
    try:
        environment_a = RecordingEnvironment()
        env_a = environment_a
        environment_b = RecordingEnvironment()
        env_b = environment_b
        env_a.reset_synthetic(root_seed_hex=seed_hex_a)
        env_b.reset_synthetic(root_seed_hex=seed_hex_b if seed_hex_b is not None else seed_hex_a)
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
        for label, environment in (("A", environment_a), ("B", environment_b)):
            if environment is not None:
                _guarded_teardown_step(
                    f"twin {label} environment shutdown", environment.shutdown, errors
                )
                core = environment._core
                _guarded_teardown_step(f"twin {label} core close", core.close, errors)
        if errors and body_error is None:
            raise errors[0]
        if errors:
            print(
                f"[m2_h] {len(errors)} twin teardown error(s) suppressed "
                "behind the original scenario error",
                file=sys.stderr,
            )


@contextlib.contextmanager
def build_single_client(seed_hex: str) -> Iterator[SyntheticEnvironmentClient]:
    """One identically-reset environment on a ``RecordingEnvironment`` with
    the same exception-safe teardown guarantee as :func:`build_twin_clients`."""
    environment: RecordingEnvironment | None = None
    body_error: BaseException | None = None
    try:
        environment = RecordingEnvironment()
        environment.reset_synthetic(root_seed_hex=seed_hex)
        yield environment
    except BaseException as exc:
        body_error = exc
        raise
    finally:
        errors: list[BaseException] = []
        if environment is not None:
            _guarded_teardown_step("environment shutdown", environment.shutdown, errors)
            core = environment._core
            _guarded_teardown_step("core close", core.close, errors)
        if errors and body_error is None:
            raise errors[0]
        if errors:
            print(
                "[m2_h] 1+ session teardown error(s) suppressed behind the original scenario error",
                file=sys.stderr,
            )


def recorded_results(environment: SyntheticEnvironmentClient) -> list[dict[str, object]]:
    core = environment._core
    assert isinstance(core, RecordingCore), "environments must be built on recording cores"
    return core.recorded_results


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


MEMBER_COUNT: Final = 2


def choose_number_member_count(request: PlayerDecisionRequestV2) -> DecisionAnswerV2:
    """Stage-count driver fixing the member-set cardinality to ``MEMBER_COUNT``.

    The synthetic ChooseCount surface spans ``{0, 3}``; answering the bare
    minimum (``0``) would pin the following choose_many bounds to
    ``{0, 0}`` and make below-minimum cardinality unreachable. Answering
    ``2`` yields a ``{2, 2}`` surface with two dense candidates, which is
    the reality check every rejection scenario asserts dynamically.
    """
    return DecisionAnswerV2("choose_number", value=MEMBER_COUNT)


ANSWER_CONSTRUCTORS: Final[dict[str, Callable[[PlayerDecisionRequestV2], DecisionAnswerV2]]] = {
    "choose_one": select_one_first_offered,
    "choose_number": choose_number_spec_minimum,
    "choose_many": select_many_first_minimum_offered,
    "order": order_reversed_offered,
}


ACCEPTED_STAGE_DRIVERS: Final[dict[str, Callable[[PlayerDecisionRequestV2], DecisionAnswerV2]]] = {
    "choose_one": select_one_first_offered,
    "choose_number": choose_number_member_count,
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


def binding_surface_names(instance: object) -> set[str]:
    """Every reachable attribute name (public AND private) that even
    mentions binding, tokens, or perspective selection."""
    return {
        name
        for name in dir(instance)
        if any(marker in name.lower() for marker in BINDING_SURFACE_MARKERS)
    }


def assert_public_surface_unchanged(
    test: unittest.TestCase,
    player: AdapterPlayerClient,
    environment: SyntheticEnvironmentClient,
) -> None:
    test.assertEqual(public_method_names(player), set(PLAYER_CLIENT_PUBLIC_METHODS))
    test.assertEqual(public_method_names(environment), set(SYNTHETIC_PUBLIC_METHODS))
    test.assertIn(TRUSTED_DIRECT_CALL_NAME, dir(environment))


def _bound_client(twins: TwinClients, player: str) -> AdapterPlayerClient:
    if player == PLAYER_ONE:
        return twins.client_a1
    if player == PLAYER_TWO:
        return twins.client_a2
    raise AssertionError(f"twin pair binds no client for player {player!r}")


def live_request_pair(
    twins: TwinClients,
    test: unittest.TestCase,
) -> tuple[PlayerDecisionRequestV2 | None, PlayerDecisionRequestV2 | None]:
    """Current P1 visible request from both routes, byte-compared.

    Returns ``(token-route request, direct-route request)``; both ``None``
    exactly when no decision is pending on either twin."""
    request_a = twins.client_a1.visible_decision()
    raw_b = payload_field(
        direct_call_result(twins.env_b, CMD_VISIBLE_DECISION, PLAYER_ONE),
        FIELD_VISIBLE_DECISION_WIRE_B64,
    )
    if request_a is None:
        test.assertIsNone(
            raw_b, "the trusted direct-call route exposes a decision the token route hides"
        )
        return None, None
    test.assertIsNotNone(raw_b, "the trusted direct-call route lacks the pending decision")
    assert raw_b is not None
    test.assertEqual(
        wire_payload_bytes(request_a),
        raw_b,
        "the two routes disagree on the live visible-decision payload",
    )
    return request_a, decode_request(raw_b)


def token_view_bytes(client: AdapterPlayerClient) -> tuple[bytes, bytes | None]:
    """Player-visible view of one bound client: (information state, decision)."""
    state = client.information_state()
    decision = client.visible_decision()
    return wire_payload_bytes(state), optional_wire_payload_bytes(decision)


def direct_view_bytes(
    environment: SyntheticEnvironmentClient, player: str
) -> tuple[bytes | None, bytes | None]:
    """Trusted-route mirror of :func:`token_view_bytes` for one player."""
    state = payload_field(
        direct_call_result(environment, CMD_INFORMATION_STATE, player),
        FIELD_INFORMATION_STATE_WIRE_B64,
    )
    decision = payload_field(
        direct_call_result(environment, CMD_VISIBLE_DECISION, player),
        FIELD_VISIBLE_DECISION_WIRE_B64,
    )
    return state, decision


ViewSlot = tuple[bytes | None, bytes | None]


def all_visible_views(twins: TwinClients) -> dict[tuple[str, str], ViewSlot]:
    """Complete player-visible surface: information-state and visible-
    decision payload bytes for BOTH players on BOTH routes."""
    return {
        (PLAYER_ONE, "token"): token_view_bytes(twins.client_a1),
        (PLAYER_TWO, "token"): token_view_bytes(twins.client_a2),
        (PLAYER_ONE, "direct"): direct_view_bytes(twins.env_b, PLAYER_ONE),
        (PLAYER_TWO, "direct"): direct_view_bytes(twins.env_b, PLAYER_TWO),
    }


def assert_views_unchanged(
    test: unittest.TestCase,
    before: tuple[bytes | None, bytes | None],
    after: tuple[bytes | None, bytes | None],
    context: str,
) -> None:
    test.assertEqual(before[0], after[0], f"{context}: information-state payload mutated")
    test.assertEqual(before[1], after[1], f"{context}: visible-decision payload mutated")


def submit_response_both_routes(
    twins: TwinClients,
    test: unittest.TestCase,
    request_a: PlayerDecisionRequestV2,
    request_b: PlayerDecisionRequestV2,
    answer: DecisionAnswerV2,
    player: str = PLAYER_ONE,
) -> tuple[PlayerStepV2, PlayerStepV2]:
    """Encode ONE answer against each route's live request, prove the two
    injections byte-identical, submit through the token route and the
    trusted direct-call route, and return the decoded ``(step_a, step_b)``
    after asserting their step payloads are byte-equal."""
    response_a = response_for(request_a, answer)
    response_b = response_for(request_b, answer)
    raw_injection = submit_response_bytes(response_a)
    test.assertEqual(
        raw_injection,
        submit_response_bytes(response_b),
        "the two routes received diverging injections for the same answer",
    )
    step_a = _bound_client(twins, player).submit(response_a)
    raw_step_b = payload_field(
        direct_call_result(twins.env_b, CMD_SUBMIT, player, raw_injection),
        FIELD_STEP_WIRE_B64,
    )
    test.assertIsNotNone(
        raw_step_b,
        f"player {player}: trusted direct-call submit result lacks its step payload",
    )
    assert raw_step_b is not None
    step_b = decode_step(raw_step_b)
    test.assertEqual(
        wire_payload_bytes(step_a),
        raw_step_b,
        f"player {player}: step payloads diverge across routes",
    )
    return step_a, step_b


def assert_submission_pair(
    test: unittest.TestCase,
    step_a: PlayerStepV2,
    step_b: PlayerStepV2,
    expected_kind: str,
    expected_code: str | None,
    context: str,
) -> None:
    for label, step in (("token route", step_a), ("direct route", step_b)):
        test.assertEqual(
            step.submission.kind,
            expected_kind,
            f"{context} ({label}): unexpected submission kind",
        )
        test.assertEqual(step.submission.code, expected_code, f"{context} ({label}): code")


def advance_accepted_stage(
    twins: TwinClients,
    test: unittest.TestCase,
    kind: str,
) -> None:
    """Drive ONE accepted transition through BOTH twins in lockstep."""
    request_a, request_b = live_request_pair(twins, test)
    assert request_a is not None and request_b is not None, f"no live request for stage {kind}"
    test.assertEqual(request_a.decision.kind, kind, "stage driving met an unexpected family")
    driver = ACCEPTED_STAGE_DRIVERS[kind]
    answer = driver(request_a)
    step_a, step_b = submit_response_both_routes(twins, test, request_a, request_b, answer)
    assert_submission_pair(test, step_a, step_b, "accepted", None, f"accepted stage {kind}")


def drive_twins_to_stage(
    twins: TwinClients,
    test: unittest.TestCase,
    stage_index: int,
) -> tuple[PlayerDecisionRequestV2, PlayerDecisionRequestV2]:
    """Advance both twins through accepted stages until the live P1 request
    is the one at ``EXPECTED_DECISION_KIND_SEQUENCE[stage_index]``."""
    for index in range(stage_index):
        advance_accepted_stage(twins, test, EXPECTED_DECISION_KIND_SEQUENCE[index])
    request_a, request_b = live_request_pair(twins, test)
    assert request_a is not None and request_b is not None, "stage has no live request"
    test.assertEqual(
        request_a.decision.kind,
        EXPECTED_DECISION_KIND_SEQUENCE[stage_index],
        "stage driving landed on the wrong family",
    )
    return request_a, request_b


def finish_lockstep_after(
    twins: TwinClients,
    test: unittest.TestCase,
    stage_index: int,
) -> None:
    """Continue lockstep from a rejected stage: a rejection leaves its own
    request live, so accept that stage first, then every remaining one,
    and prove both routes rest together with no pending decision."""
    for index in range(stage_index, len(EXPECTED_DECISION_KIND_SEQUENCE)):
        advance_accepted_stage(twins, test, EXPECTED_DECISION_KIND_SEQUENCE[index])
    request_a, request_b = live_request_pair(twins, test)
    test.assertIsNone(request_a, "a decision remained after completing the chain")
    test.assertIsNone(request_b, "trusted route kept a decision after completing the chain")
