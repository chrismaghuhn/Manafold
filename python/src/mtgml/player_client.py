from __future__ import annotations

from typing import Protocol

from .decision import DecisionResponseV2, PlayerDecisionRequestV2
from .observation import (
    ObservationEnvelope,
    PlayerInformationStateV2,
    PlayerStepV2,
)


class PlayerClient(Protocol):
    """Perspective-bound, untrusted player capability."""

    def observation(self) -> ObservationEnvelope: ...

    def information_state(self) -> PlayerInformationStateV2: ...

    def visible_decision(self) -> PlayerDecisionRequestV2 | None: ...

    def submit(self, response: DecisionResponseV2) -> PlayerStepV2: ...
