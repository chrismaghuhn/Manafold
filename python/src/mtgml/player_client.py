from __future__ import annotations

from typing import Protocol

from .decision import DecisionResponseV2, PlayerDecisionRequestV2
from .decision_v3 import PlayerDecisionRequestV3
from .observation import (
    ObservationEnvelope,
    PlayerInformationStateV2,
    PlayerStepV2,
    PlayerStepV3,
)


class PlayerClient(Protocol):
    """Current perspective-bound player capability on the successor runtime."""

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
