from __future__ import annotations

from typing import Protocol

from .decision import DecisionResponse, PlayerDecisionRequest
from .observation import InformationStateEnvelope, ObservationEnvelope, PlayerStep


class PlayerClient(Protocol):
    """Perspective-bound, untrusted player capability."""

    def observation(self) -> ObservationEnvelope: ...

    def information_state(self) -> InformationStateEnvelope: ...

    def visible_decision(self) -> PlayerDecisionRequest | None: ...

    def submit(self, response: DecisionResponse) -> PlayerStep: ...
