from __future__ import annotations

import json
from collections.abc import Callable
from typing import TypeVar

from ._events_v4 import ObservedEventEnvelopeV4
from ._magic_basic_land_observation_v1 import MagicBasicLandObservationV1
from ._player_step_v4 import PlayerStepV4
from .canonical import canonical_json_bytes
from .decision import (
    DecisionResponseV3,
)
from .decision_v4 import PlayerDecisionRequestV4
from .episode import EpisodeStatus
from .errors import WireError
from .observation import (
    InformationStateDigestInputV3,
    MagicSharedExecutionObservationV1,
    ObservationEnvelopeV2,
    PlayerInformationStateV3,
)
from .observation import (
    compute_information_state_digest_v3 as _compute_information_state_digest_v3,
)
from .replay import (
    AuthoritativeReplayV8,
    ReplayManifestV8,
    ReplayStepV8,
)

T = TypeVar("T")

_DECODERS: dict[str, Callable[[object], object]] = {
    "player-decision-request.v4": PlayerDecisionRequestV4.from_wire,
    "decision-response.v3": DecisionResponseV3.from_wire,
    "episode-status.v1": EpisodeStatus.from_wire,
    "observation-envelope.v2": ObservationEnvelopeV2.from_wire,
    "information-state-envelope.v3": PlayerInformationStateV3.from_wire,
    "magic-shared-execution-observation.v1": MagicSharedExecutionObservationV1.from_wire,
    "observed-event-envelope.v4": ObservedEventEnvelopeV4.from_wire,
    "player-step.v4": PlayerStepV4.from_wire,
    "magic-basic-land-observation.v1": MagicBasicLandObservationV1.from_wire,
    "replay-manifest.v8": ReplayManifestV8.from_wire,
    "replay-step.v8": ReplayStepV8.from_wire,
    "authoritative-replay.v8": AuthoritativeReplayV8.from_wire,
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

    def reject_duplicate_pairs(pairs: list[tuple[str, object]]) -> dict[str, object]:
        result: dict[str, object] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate object key")
            result[key] = value
        return result

    try:
        raw = json.loads(payload.decode("utf-8"), object_pairs_hook=reject_duplicate_pairs)
    except (UnicodeDecodeError, ValueError) as exc:
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


def compute_information_state_digest_v3(
    input_value: InformationStateDigestInputV3,
) -> tuple[bytes, str]:
    return _compute_information_state_digest_v3(input_value)
