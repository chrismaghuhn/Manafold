from __future__ import annotations

from typing import Protocol

from .decision import DecisionResponseV3
from .decision_v4 import PlayerDecisionRequestV4
from .observation import ObservationEnvelopeV2, PlayerInformationStateV3, PlayerStepV4


class PlayerClient(Protocol):
    """Current perspective-bound player capability on the V8 runtime."""

    def observation(self) -> ObservationEnvelopeV2: ...

    def information_state(self) -> PlayerInformationStateV3: ...

    def visible_decision(self) -> PlayerDecisionRequestV4 | None: ...

    def submit(self, response: DecisionResponseV3) -> PlayerStepV4: ...
