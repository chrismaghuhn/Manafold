from __future__ import annotations

from itertools import pairwise

from .episode import EpisodeStatus
from .errors import WireError


def _validate_status_for_players(status: EpisodeStatus, players: set[int]) -> None:
    if status.kind == "running":
        return
    ordered = [outcome.player for outcome in status.players]
    if any(left >= right for left, right in pairwise(ordered)):
        raise WireError("semantic.replay_manifest", "status players are not in canonical order")
    actual = {outcome.player for outcome in status.players}
    if actual != players:
        raise WireError(
            "semantic.replay_manifest", "status does not cover the manifest player universe"
        )
