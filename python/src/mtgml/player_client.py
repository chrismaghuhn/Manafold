from __future__ import annotations

from typing import Protocol

from .decision import DecisionResponseV3
from .decision_v4 import PlayerDecisionRequestV4
from .observation import ObservationEnvelope, PlayerInformationState, PlayerStepV4


class PlayerClient(Protocol):
    """Current perspective-bound player capability on the V8 runtime."""

    def observation(self) -> ObservationEnvelope: ...

    def information_state(self) -> PlayerInformationState: ...

    def visible_decision(self) -> PlayerDecisionRequestV4 | None: ...

    def submit(self, response: DecisionResponseV3) -> PlayerStepV4: ...
