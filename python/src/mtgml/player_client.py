from __future__ import annotations

from typing import Protocol

from .decision import DecisionResponseV2, DecisionResponseV3, PlayerDecisionRequestV2
from .decision_v3 import PlayerDecisionRequestV3
from .decision_v4 import PlayerDecisionRequestV4
from .observation import (
    ObservationEnvelope,
    ObservationEnvelopeV2,
    PlayerInformationStateV2,
    PlayerInformationStateV3,
    PlayerStepV2,
    PlayerStepV3,
    PlayerStepV4,
)


class PlayerClient(Protocol):
    """Current perspective-bound player capability on the V8 runtime."""

    def observation(self) -> ObservationEnvelopeV2: ...

    def information_state(self) -> PlayerInformationStateV3: ...

    def visible_decision(self) -> PlayerDecisionRequestV4 | None: ...

    def submit(self, response: DecisionResponseV3) -> PlayerStepV4: ...


class HistoricalPlayerClientV3(Protocol):
    """Historical perspective-bound client for the V7/V3 runtime family."""

    def observation(self) -> ObservationEnvelope: ...

    def information_state(self) -> PlayerInformationStateV2: ...

    def visible_decision(self) -> PlayerDecisionRequestV3 | None: ...

    def submit(self, response: DecisionResponseV2) -> PlayerStepV3: ...


class HistoricalPlayerClientV2(Protocol):
    """Read-only type contract for the retained M2 adapter surface."""

    def observation(self) -> ObservationEnvelope: ...

    def information_state(self) -> PlayerInformationStateV2: ...

    def visible_decision(self) -> PlayerDecisionRequestV2 | None: ...

    def submit(self, response: DecisionResponseV2) -> PlayerStepV2: ...
