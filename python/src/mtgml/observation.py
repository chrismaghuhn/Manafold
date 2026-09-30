import base64
import hashlib
from dataclasses import dataclass
from itertools import pairwise

from ._events_v3 import ObservedEventV3
from ._events_v4 import (
    OBSERVED_EVENT_SCHEMA_V4,
    ObservedEventEnvelopeV4,
    ObservedEventV4,
)
from ._generated_contract_vocab import ZONE_KINDS
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
from ._magic_basic_land_observation_v1 import (
    MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1,
    AttachmentObservationV1,
    CounterObservationV1,
    FaceObservationV1,
    MagicBasicLandObservationV1,
    ManaPoolObservationV1,
)
from ._magic_observation import (
    MagicCompletedOrder,
    MagicPendingSbaOrdering,
)
from ._observation_v1 import (
    OBSERVATION_SCHEMA,
    ObservationEnvelope,
    observation_digest_from_payload,
)
from ._player_step_v2 import (
    PLAYER_SUBMISSION_CODES,
    PlayerStepSubmissionV1,
)
from ._player_step_v4 import PLAYER_STEP_SCHEMA_V4, PlayerStepV4
from ._synthetic_observation import (
    SYNTHETIC_BEGINNING_STEPS,
    SYNTHETIC_COMBAT_STEPS,
    SYNTHETIC_ENDING_STEPS,
    SYNTHETIC_PRIORITY_KINDS,
    SYNTHETIC_TURN_KINDS,
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
from .magic_shared_execution_observation_v1 import (
    MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1,
    ActivatedAbilityStackItemV1,
    GrantKeywordV1,
    MagicSharedExecutionObservationV1,
    PowerToughnessDeltaV1,
    PublicModeV1,
    PublicTemporaryEffectV1,
    SpellStackItemV1,
    TriggeredAbilityStackItemV1,
    UntilEndOfTurnV1,
)
from .observation_v3 import (
    INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3,
    INFORMATION_STATE_SCHEMA_V3,
    OBSERVATION_SCHEMA_V2,
    InformationStateDigestInputV3,
    ObservationEnvelopeV2,
    PlayerInformationStateV3,
    compute_information_state_digest_v3,
)

__all__ = [
    "INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3",
    "INFORMATION_STATE_SCHEMA_V2",
    "INFORMATION_STATE_SCHEMA_V3",
    "MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1",
    "MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1",
    "OBSERVATION_SCHEMA",
    "OBSERVATION_SCHEMA_V2",
    "OBSERVED_EVENT_SCHEMA_V4",
    "PLAYER_STEP_SCHEMA_V4",
    "PLAYER_SUBMISSION_CODES",
    "SYNTHETIC_BEGINNING_STEPS",
    "SYNTHETIC_COMBAT_STEPS",
    "SYNTHETIC_ENDING_STEPS",
    "SYNTHETIC_PRIORITY_KINDS",
    "SYNTHETIC_TURN_KINDS",
    "ZONE_KINDS",
    "ActivatedAbilityStackItemV1",
    "AttachmentObservationV1",
    "CounterObservationV1",
    "EpisodeStatus",
    "FaceObservationV1",
    "GrantKeywordV1",
    "InformationStateDigestInputV2",
    "InformationStateDigestInputV3",
    "MagicBasicLandObservationV1",
    "MagicCompletedOrder",
    "MagicPendingSbaOrdering",
    "MagicSharedExecutionObservationV1",
    "ManaPoolObservationV1",
    "ObservationEnvelope",
    "ObservationEnvelopeV2",
    "ObservedEventEnvelopeV4",
    "ObservedEventV3",
    "ObservedEventV4",
    "PlayerDecisionRequest",
    "PlayerDecisionRequestV2",
    "PlayerInformationStateV2",
    "PlayerInformationStateV3",
    "PlayerKnowledgeInvalidationV1",
    "PlayerKnowledgeProvenanceV1",
    "PlayerKnownLocationFactV1",
    "PlayerKnownLocationV1",
    "PlayerKnownObjectV1",
    "PlayerStepSubmissionV1",
    "PlayerStepV4",
    "PowerToughnessDeltaV1",
    "PublicModeV1",
    "PublicTemporaryEffectV1",
    "SpellStackItemV1",
    "SyntheticPriority",
    "SyntheticTurnPosition",
    "TriggeredAbilityStackItemV1",
    "UntilEndOfTurnV1",
    "WireError",
    "base64",
    "compute_information_state_digest_v3",
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
