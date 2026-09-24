import base64
import hashlib
from dataclasses import dataclass
from itertools import pairwise

from ._events_v2 import (
    OBSERVED_EVENT_SCHEMA_V2,
    ObservedEventEnvelopeV2,
    ObservedEventV2,
)
from ._generated_contract_vocab import OBSERVED_EVENT_KINDS, ZONE_KINDS
from ._information_v2 import (
    INFORMATION_STATE_SCHEMA_V2,
    InformationStateDigestInputV2,
    PlayerInformationStateV2,
)
from ._knowledge import (
    PlayerKnowledgeInvalidationV1,
    PlayerKnowledgeProvenanceV1,
    PlayerKnownLocationFactV1,
    PlayerKnownLocationV1,
    PlayerKnownObjectV1,
)
from ._magic_combat_observation import (
    MAGIC_OBSERVATION_SCHEMA_V2,
    MagicCombatBlockerAssignmentV2,
    MagicCombatParticipationV2,
    MagicObservationV2,
)
from ._magic_combat_observation_v3 import (
    MAGIC_OBSERVATION_SCHEMA_V3,
    MagicCombatBlockerAssignmentV3,
    MagicCombatParticipationV3,
    MagicObservationV3,
)
from ._magic_observation import (
    MAGIC_OBSERVATION_SCHEMA_V1,
    MagicCompletedOrder,
    MagicObservation,
    MagicPendingSbaOrdering,
)
from ._observation_v1 import (
    INFORMATION_STATE_SCHEMA,
    OBSERVATION_SCHEMA,
    PLAYER_STEP_SCHEMA,
    InformationStateEnvelope,
    ObservationEnvelope,
    PlayerStep,
    observation_digest_from_payload,
)
from ._player_step_v2 import (
    PLAYER_STEP_SCHEMA_V2,
    PLAYER_SUBMISSION_CODES,
    PlayerStepSubmissionV1,
    PlayerStepV2,
)
from ._synthetic_observation import (
    SYNTHETIC_BEGINNING_STEPS,
    SYNTHETIC_COMBAT_STEPS,
    SYNTHETIC_ENDING_STEPS,
    SYNTHETIC_OBSERVATION_SCHEMA_V1,
    SYNTHETIC_PRIORITY_KINDS,
    SYNTHETIC_TURN_KINDS,
    SyntheticObservation,
    SyntheticPriority,
    SyntheticTurnPosition,
)
from .canonical import (
    parse_u64_number,
    parse_uint,
    require_canonical_base64,
    require_digest,
    require_exact_keys,
    require_nonempty,
    uint_wire,
)
from .decision import PlayerDecisionRequest, PlayerDecisionRequestV2
from .episode import EpisodeStatus
from .errors import WireError
from .events import ObservedEventEnvelope

__all__ = [
    "INFORMATION_STATE_SCHEMA",
    "INFORMATION_STATE_SCHEMA_V2",
    "MAGIC_OBSERVATION_SCHEMA_V1",
    "MAGIC_OBSERVATION_SCHEMA_V2",
    "MAGIC_OBSERVATION_SCHEMA_V3",
    "OBSERVATION_SCHEMA",
    "OBSERVED_EVENT_KINDS",
    "OBSERVED_EVENT_SCHEMA_V2",
    "PLAYER_STEP_SCHEMA",
    "PLAYER_STEP_SCHEMA_V2",
    "PLAYER_SUBMISSION_CODES",
    "SYNTHETIC_BEGINNING_STEPS",
    "SYNTHETIC_COMBAT_STEPS",
    "SYNTHETIC_ENDING_STEPS",
    "SYNTHETIC_OBSERVATION_SCHEMA_V1",
    "SYNTHETIC_PRIORITY_KINDS",
    "SYNTHETIC_TURN_KINDS",
    "ZONE_KINDS",
    "EpisodeStatus",
    "InformationStateDigestInputV2",
    "InformationStateEnvelope",
    "MagicCombatBlockerAssignmentV2",
    "MagicCombatBlockerAssignmentV3",
    "MagicCombatParticipationV2",
    "MagicCombatParticipationV3",
    "MagicCompletedOrder",
    "MagicObservation",
    "MagicObservationV2",
    "MagicObservationV3",
    "MagicPendingSbaOrdering",
    "ObservationEnvelope",
    "ObservedEventEnvelope",
    "ObservedEventEnvelopeV2",
    "ObservedEventV2",
    "PlayerDecisionRequest",
    "PlayerDecisionRequestV2",
    "PlayerInformationStateV2",
    "PlayerKnowledgeInvalidationV1",
    "PlayerKnowledgeProvenanceV1",
    "PlayerKnownLocationFactV1",
    "PlayerKnownLocationV1",
    "PlayerKnownObjectV1",
    "PlayerStep",
    "PlayerStepSubmissionV1",
    "PlayerStepV2",
    "SyntheticObservation",
    "SyntheticPriority",
    "SyntheticTurnPosition",
    "WireError",
    "base64",
    "dataclass",
    "hashlib",
    "observation_digest_from_payload",
    "pairwise",
    "parse_u64_number",
    "parse_uint",
    "require_canonical_base64",
    "require_digest",
    "require_exact_keys",
    "require_nonempty",
    "uint_wire",
]
