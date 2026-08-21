from __future__ import annotations

import json
import hashlib
from collections.abc import Callable
from typing import TypeVar

from .canonical import canonical_json_bytes
from .decision import (
    DecisionResponse,
    DecisionResponseV2,
    PlayerDecisionRequest,
    PlayerDecisionRequestV2,
)
from .episode import EpisodeStatus
from .errors import WireError
from .events import ObservedEventEnvelope
from .observation import (
    InformationStateDigestInputV2,
    InformationStateEnvelope,
    ObservationEnvelope,
    ObservedEventEnvelopeV2,
    PlayerInformationStateV2,
    PlayerStep,
    PlayerStepV2,
)
from .replay import (
    AuthoritativeReplayV1,
    AuthoritativeReplayV2,
    AuthoritativeReplayV3,
    ReplayManifestV1,
    ReplayManifestV2,
    ReplayManifestV3,
)

T = TypeVar("T")

_DECODERS: dict[str, Callable[[object], object]] = {
    "player-decision-request.v1": PlayerDecisionRequest.from_wire,
    "decision-response.v1": DecisionResponse.from_wire,
    "player-decision-request.v2": PlayerDecisionRequestV2.from_wire,
    "decision-response.v2": DecisionResponseV2.from_wire,
    "episode-status.v1": EpisodeStatus.from_wire,
    "observed-event-envelope.v1": ObservedEventEnvelope.from_wire,
    "observation-envelope.v1": ObservationEnvelope.from_wire,
    "information-state-envelope.v1": InformationStateEnvelope.from_wire,
    "player-step.v1": PlayerStep.from_wire,
    "information-state-envelope.v2": PlayerInformationStateV2.from_wire,
    "information-state-digest-input.v2": InformationStateDigestInputV2.from_wire,
    "observed-event-envelope.v2": ObservedEventEnvelopeV2.from_wire,
    "player-step.v2": PlayerStepV2.from_wire,
    "replay-manifest.v1": ReplayManifestV1.from_wire,
    "authoritative-replay.v1": AuthoritativeReplayV1.from_wire,
    "replay-manifest.v2": ReplayManifestV2.from_wire,
    "authoritative-replay.v2": AuthoritativeReplayV2.from_wire,
    "replay-manifest.v3": ReplayManifestV3.from_wire,
    "authoritative-replay.v3": AuthoritativeReplayV3.from_wire,
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


def compute_information_state_digest_v2(
    input_value: InformationStateDigestInputV2,
) -> tuple[bytes, str]:
    payload = encode_canonical(input_value)
    digest = hashlib.sha256(
        b"mtgml.information-state-digest.v2\0" + payload
    ).hexdigest()
    return payload, digest
