from __future__ import annotations

import json
from collections.abc import Callable
from typing import Any, TypeVar

from .canonical import canonical_json_bytes
from .decision import DecisionResponse, PlayerDecisionRequest
from .episode import EpisodeStatus
from .errors import WireError
from .events import ObservedEventEnvelope
from .observation import InformationStateEnvelope, ObservationEnvelope, PlayerStep
from .replay import AuthoritativeReplayV1, ReplayManifestV1

T = TypeVar("T")

_DECODERS: dict[str, Callable[[object], object]] = {
    "player-decision-request.v1": PlayerDecisionRequest.from_wire,
    "decision-response.v1": DecisionResponse.from_wire,
    "episode-status.v1": EpisodeStatus.from_wire,
    "observed-event-envelope.v1": ObservedEventEnvelope.from_wire,
    "observation-envelope.v1": ObservationEnvelope.from_wire,
    "information-state-envelope.v1": InformationStateEnvelope.from_wire,
    "player-step.v1": PlayerStep.from_wire,
    "replay-manifest.v1": ReplayManifestV1.from_wire,
    "authoritative-replay.v1": AuthoritativeReplayV1.from_wire,
}


def encode_canonical(value: object) -> bytes:
    to_wire = getattr(value, "to_wire", None)
    if to_wire is None:
        raise WireError("encode.serialization", "value has no public wire encoder")
    return canonical_json_bytes(to_wire())


def decode_canonical(contract: str, payload: bytes) -> object:
    decoder = _DECODERS.get(contract)
    if decoder is None:
        raise WireError("fixture.unknown_contract", f"unknown contract {contract}")
    try:
        raw = json.loads(payload.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise WireError("decode.invalid_json", str(exc)) from exc
    try:
        result = decoder(raw)
    except WireError:
        raise
    except (TypeError, ValueError, KeyError) as exc:
        raise WireError("decode.invalid_json", str(exc)) from exc
    canonical = encode_canonical(result)
    if canonical != payload:
        raise WireError("decode.non_canonical_json", "wire bytes are not canonical")
    return result
