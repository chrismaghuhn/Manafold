"""Rules-free DTOs for the detached Replay V8 identity family."""

from __future__ import annotations

from dataclasses import dataclass

from ._replay_common import DeckIdentityV1, KernelIdentityV1
from ._replay_v2 import RandomnessIdentityV2
from ._replay_v4 import CheckpointCodecIdentityV4, EnvironmentLimitCountersV4
from ._replay_v5 import ExecutionIdentityV1
from ._replay_v6 import _validate_status_for_players
from ._replay_v7 import SemanticContractMaterialV7
from .canonical import (
    parse_u64_number,
    parse_uint,
    require_digest,
    require_exact_keys,
    require_nonempty,
    uint_wire,
)
from .decision import DecisionResponseV3
from .episode import EpisodeStatus
from .errors import WireError
from .persistence import calculate_checkpoint_digest_v8

REPLAY_MANIFEST_SCHEMA_V8 = "replay-manifest.v8"
REPLAY_FILE_SCHEMA_V8 = "authoritative-replay.v8"
REPLAY_STEP_SCHEMA_V8 = "replay-step.v8"
CHECKPOINT_CODEC_ID_V8 = "in-memory-reference"
CHECKPOINT_CODEC_VERSION_V8 = "8"
SHARED_OBSERVATION_CODEC_V1 = "magic-shared-execution-observation.v1"


@dataclass(frozen=True, slots=True)
class ReplaySchemaVersionsV8:
    observation: str
    observation_payload_codec: str
    information_state: str
    decision: str
    decision_response: str
    observed_event: str
    player_step: str
    replay_step: str

    @classmethod
    def from_wire(cls, value: object) -> ReplaySchemaVersionsV8:
        obj = require_exact_keys(
            value,
            {
                "observation",
                "observation_payload_codec",
                "information_state",
                "decision",
                "decision_response",
                "observed_event",
                "player_step",
                "replay_step",
            },
        )
        return cls(**{key: require_nonempty(obj[key], key) for key in obj})

    def validate(self) -> None:
        expected = {
            "observation": "observation-envelope.v2",
            "observation_payload_codec": SHARED_OBSERVATION_CODEC_V1,
            "information_state": "information-state-envelope.v3",
            "decision": "player-decision-request.v4",
            "decision_response": "decision-response.v3",
            "observed_event": "observed-event-envelope.v4",
            "player_step": "player-step.v4",
            "replay_step": REPLAY_STEP_SCHEMA_V8,
        }
        if self.to_wire() != expected:
            raise WireError("semantic.replay_manifest", "Replay V8 child identity chain differs")

    def to_wire(self) -> dict[str, object]:
        self.validate_values()
        return {
            "observation": self.observation,
            "observation_payload_codec": self.observation_payload_codec,
            "information_state": self.information_state,
            "decision": self.decision,
            "decision_response": self.decision_response,
            "observed_event": self.observed_event,
            "player_step": self.player_step,
            "replay_step": self.replay_step,
        }

    def validate_values(self) -> None:
        for name, value in self.to_values().items():
            require_nonempty(value, name)

    def to_values(self) -> dict[str, str]:
        return {
            "observation": self.observation,
            "observation_payload_codec": self.observation_payload_codec,
            "information_state": self.information_state,
            "decision": self.decision,
            "decision_response": self.decision_response,
            "observed_event": self.observed_event,
            "player_step": self.player_step,
            "replay_step": self.replay_step,
        }


@dataclass(frozen=True, slots=True)
class InitialEnvironmentIdentityV8:
    state_revision: int
    full_state_digest: str
    episode_status: EpisodeStatus
    environment_limit_counters: EnvironmentLimitCountersV4
    checkpoint_codec_identity: CheckpointCodecIdentityV4
    checkpoint_digest: str
    execution_identity: ExecutionIdentityV1

    @classmethod
    def from_wire(cls, value: object) -> InitialEnvironmentIdentityV8:
        obj = require_exact_keys(
            value,
            {
                "state_revision",
                "full_state_digest",
                "episode_status",
                "environment_limit_counters",
                "checkpoint_codec_identity",
                "checkpoint_digest",
                "execution_identity",
            },
        )
        result = cls(
            parse_uint(obj["state_revision"]),
            require_digest(obj["full_state_digest"]),
            EpisodeStatus.from_wire(obj["episode_status"]),
            EnvironmentLimitCountersV4.from_wire(obj["environment_limit_counters"]),
            CheckpointCodecIdentityV4.from_wire(obj["checkpoint_codec_identity"]),
            require_digest(obj["checkpoint_digest"]),
            ExecutionIdentityV1.from_wire(obj["execution_identity"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        self.episode_status.to_wire()
        self.environment_limit_counters.to_wire()
        if (
            self.checkpoint_codec_identity.codec_id != CHECKPOINT_CODEC_ID_V8
            or self.checkpoint_codec_identity.semantic_version != CHECKPOINT_CODEC_VERSION_V8
        ):
            raise WireError("semantic.replay_manifest", "unsupported checkpoint V8 identity")
        expected = calculate_checkpoint_digest_v8(
            self.full_state_digest,
            self.episode_status,
            self.environment_limit_counters.as_dict(),
            self.checkpoint_codec_identity.codec_id,
            self.checkpoint_codec_identity.semantic_version,
            self.execution_identity.program_kind,
            self.execution_identity.semantic_contract_id,
        )
        if expected != self.checkpoint_digest:
            raise WireError("semantic.replay_manifest", "checkpoint V8 digest does not match")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "state_revision": uint_wire(self.state_revision),
            "full_state_digest": require_digest(self.full_state_digest),
            "episode_status": self.episode_status.to_wire(),
            "environment_limit_counters": self.environment_limit_counters.to_wire(),
            "checkpoint_codec_identity": self.checkpoint_codec_identity.to_wire(),
            "checkpoint_digest": require_digest(self.checkpoint_digest),
            "execution_identity": self.execution_identity.to_wire(),
        }


@dataclass(frozen=True, slots=True)
class ReplayManifestV8:
    schema_version: str
    engine_build: str
    kernel: KernelIdentityV1
    rules_snapshot: str
    format_policy_snapshot: str
    oracle_snapshot: str
    card_bundle: str
    schemas: ReplaySchemaVersionsV8
    randomness: RandomnessIdentityV2
    decks: tuple[DeckIdentityV1, ...]
    initial_identity: InitialEnvironmentIdentityV8
    execution_identity: ExecutionIdentityV1
    semantic_contract: SemanticContractMaterialV7

    @classmethod
    def from_wire(cls, value: object) -> ReplayManifestV8:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "engine_build",
                "kernel",
                "rules_snapshot",
                "format_policy_snapshot",
                "oracle_snapshot",
                "card_bundle",
                "schemas",
                "randomness",
                "decks",
                "initial_identity",
                "execution_identity",
                "semantic_contract",
            },
        )
        if obj["schema_version"] != REPLAY_MANIFEST_SCHEMA_V8 or not isinstance(obj["decks"], list):
            raise WireError("decode.invalid_json", "unsupported ReplayManifestV8")
        from ._replay_common import DeckIdentityV1, KernelIdentityV1

        result = cls(
            REPLAY_MANIFEST_SCHEMA_V8,
            require_nonempty(obj["engine_build"], "engine_build"),
            KernelIdentityV1.from_wire(obj["kernel"]),
            require_nonempty(obj["rules_snapshot"], "rules_snapshot"),
            require_nonempty(obj["format_policy_snapshot"], "format_policy_snapshot"),
            require_nonempty(obj["oracle_snapshot"], "oracle_snapshot"),
            require_nonempty(obj["card_bundle"], "card_bundle"),
            ReplaySchemaVersionsV8.from_wire(obj["schemas"]),
            RandomnessIdentityV2.from_wire(obj["randomness"]),
            tuple(DeckIdentityV1.from_wire(item) for item in obj["decks"]),
            InitialEnvironmentIdentityV8.from_wire(obj["initial_identity"]),
            ExecutionIdentityV1.from_wire(obj["execution_identity"]),
            SemanticContractMaterialV7.from_wire(obj["semantic_contract"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if self.schema_version != REPLAY_MANIFEST_SCHEMA_V8:
            raise WireError("semantic.replay_manifest", "unsupported ReplayManifestV8")
        self.schemas.validate()
        self.randomness.to_wire()
        self.semantic_contract.validate()
        if self.randomness.contract_id != "mtgml.rng.v1":
            raise WireError("semantic.replay_manifest", "unsupported RNG contract")
        if (
            self.execution_identity.semantic_contract_id
            != self.semantic_contract.semantic_contract_id
        ):
            raise WireError("semantic.replay_manifest", "execution identity differs")
        if self.initial_identity.execution_identity != self.execution_identity:
            raise WireError("semantic.replay_manifest", "initial execution identity differs")
        authority = self.semantic_contract.rules_manifest.get("rules_authority")
        if not isinstance(authority, dict):
            raise WireError("semantic.replay_manifest", "rules authority is malformed")
        program_authority = {
            "magic_rules": "comprehensive_rules",
            "synthetic_rules_compat": "synthetic_legacy",
        }
        if program_authority.get(self.execution_identity.program_kind) != authority.get("variant"):
            raise WireError(
                "semantic.replay_manifest", "execution program and rules authority do not match"
            )
        if (
            authority.get("variant") == "comprehensive_rules"
            and authority.get("snapshot_id") != self.rules_snapshot
        ):
            raise WireError("semantic.replay_manifest", "rules snapshot does not match")
        players: list[int] = []
        for deck in self.decks:
            deck.to_wire()
            players.append(deck.player)
        if not players:
            raise WireError("semantic.replay_manifest", "manifest has no decks")
        if players != sorted(set(players)):
            raise WireError("semantic.replay_manifest", "decks are not uniquely ordered")
        _validate_status_for_players(self.initial_identity.episode_status, set(players))

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "schema_version": REPLAY_MANIFEST_SCHEMA_V8,
            "engine_build": self.engine_build,
            "kernel": self.kernel.to_wire(),
            "rules_snapshot": self.rules_snapshot,
            "format_policy_snapshot": self.format_policy_snapshot,
            "oracle_snapshot": self.oracle_snapshot,
            "card_bundle": self.card_bundle,
            "schemas": self.schemas.to_wire(),
            "randomness": self.randomness.to_wire(),
            "decks": [deck.to_wire() for deck in self.decks],
            "initial_identity": self.initial_identity.to_wire(),
            "execution_identity": self.execution_identity.to_wire(),
            "semantic_contract": self.semantic_contract.to_wire(),
        }


@dataclass(frozen=True, slots=True)
class ReplayStepV8:
    step_index: int
    actor: int
    checkpoint_digest_before: str
    state_revision_before: int
    response: DecisionResponseV3
    accepted: bool
    state_revision_after: int
    full_state_digest_after: str
    episode_status_after: EpisodeStatus
    environment_limit_counters_after: EnvironmentLimitCountersV4
    checkpoint_digest_after: str

    @classmethod
    def from_wire(cls, value: object) -> ReplayStepV8:
        obj = require_exact_keys(
            value,
            {
                "step_index",
                "actor",
                "checkpoint_digest_before",
                "state_revision_before",
                "response",
                "accepted",
                "state_revision_after",
                "full_state_digest_after",
                "episode_status_after",
                "environment_limit_counters_after",
                "checkpoint_digest_after",
            },
        )
        if not isinstance(obj["accepted"], bool):
            raise WireError("decode.invalid_json", "accepted must be boolean")
        return cls(
            parse_u64_number(obj["step_index"]),
            parse_uint(obj["actor"]),
            require_digest(obj["checkpoint_digest_before"]),
            parse_uint(obj["state_revision_before"]),
            DecisionResponseV3.from_wire(obj["response"]),
            obj["accepted"],
            parse_uint(obj["state_revision_after"]),
            require_digest(obj["full_state_digest_after"]),
            EpisodeStatus.from_wire(obj["episode_status_after"]),
            EnvironmentLimitCountersV4.from_wire(obj["environment_limit_counters_after"]),
            require_digest(obj["checkpoint_digest_after"]),
        ).validated()

    def validated(self) -> ReplayStepV8:
        self.response.validate()
        self.episode_status_after.to_wire()
        self.environment_limit_counters_after.to_wire()
        if self.accepted and self.state_revision_after <= self.state_revision_before:
            raise WireError("semantic.replay_manifest", "accepted step did not advance revision")
        if not self.accepted and self.state_revision_after != self.state_revision_before:
            raise WireError("semantic.replay_manifest", "rejected step changed revision")
        return self

    def to_wire(self) -> dict[str, object]:
        return {
            "step_index": self.step_index,
            "actor": uint_wire(self.actor),
            "checkpoint_digest_before": require_digest(self.checkpoint_digest_before),
            "state_revision_before": uint_wire(self.state_revision_before),
            "response": self.response.to_wire(),
            "accepted": self.accepted,
            "state_revision_after": uint_wire(self.state_revision_after),
            "full_state_digest_after": require_digest(self.full_state_digest_after),
            "episode_status_after": self.episode_status_after.to_wire(),
            "environment_limit_counters_after": self.environment_limit_counters_after.to_wire(),
            "checkpoint_digest_after": require_digest(self.checkpoint_digest_after),
        }


@dataclass(frozen=True, slots=True)
class AuthoritativeReplayV8:
    schema_version: str
    manifest: ReplayManifestV8
    steps: tuple[ReplayStepV8, ...]
    final_identity: InitialEnvironmentIdentityV8

    @classmethod
    def from_wire(cls, value: object) -> AuthoritativeReplayV8:
        obj = require_exact_keys(value, {"schema_version", "manifest", "steps", "final_identity"})
        if obj["schema_version"] != REPLAY_FILE_SCHEMA_V8 or not isinstance(obj["steps"], list):
            raise WireError("decode.invalid_json", "unsupported AuthoritativeReplayV8")
        try:
            manifest = ReplayManifestV8.from_wire(obj["manifest"])
        except WireError as exc:
            if exc.code == "semantic.replay_manifest":
                raise WireError("semantic.replay", exc.message) from exc
            raise
        result = cls(
            REPLAY_FILE_SCHEMA_V8,
            manifest,
            tuple(ReplayStepV8.from_wire(item) for item in obj["steps"]),
            InitialEnvironmentIdentityV8.from_wire(obj["final_identity"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        self.manifest.validate()
        initial = self.manifest.initial_identity
        if initial.execution_identity != self.manifest.execution_identity or (
            initial.execution_identity != self.final_identity.execution_identity
        ):
            raise WireError("semantic.replay_manifest", "execution identity chain differs")
        players = {deck.player for deck in self.manifest.decks}
        previous = initial
        for index, step in enumerate(self.steps):
            if previous.episode_status.kind != "running":
                raise WireError("semantic.replay", "transition follows a closed episode")
            step.validated()
            if (
                step.step_index != index
                or step.actor not in players
                or step.checkpoint_digest_before != previous.checkpoint_digest
                or step.state_revision_before != previous.state_revision
            ):
                raise WireError("semantic.replay_manifest", "replay step chain is discontinuous")
            step.response.validate()
            if not step.accepted:
                if (
                    step.state_revision_after != previous.state_revision
                    or step.full_state_digest_after != previous.full_state_digest
                    or step.episode_status_after != previous.episode_status
                    or step.environment_limit_counters_after != previous.environment_limit_counters
                    or step.checkpoint_digest_after != previous.checkpoint_digest
                ):
                    raise WireError("semantic.replay_manifest", "rejected step mutated identity")
            elif step.state_revision_after <= previous.state_revision:
                raise WireError(
                    "semantic.replay_manifest", "accepted step did not advance revision"
                )
            else:
                before = previous.environment_limit_counters
                after = step.environment_limit_counters_after
                if (
                    after.decisions_submitted != before.decisions_submitted + 1
                    or after.accepted_transitions != before.accepted_transitions + 1
                    or after.rule_events_emitted < before.rule_events_emitted
                    or after.resource_units_consumed < before.resource_units_consumed
                    or after.wall_clock_elapsed_millis < before.wall_clock_elapsed_millis
                ):
                    raise WireError("semantic.replay_manifest", "accepted counters do not progress")
            previous = InitialEnvironmentIdentityV8(
                step.state_revision_after,
                step.full_state_digest_after,
                step.episode_status_after,
                step.environment_limit_counters_after,
                previous.checkpoint_codec_identity,
                step.checkpoint_digest_after,
                previous.execution_identity,
            )
            previous.validate()
            _validate_status_for_players(previous.episode_status, players)
        if previous != self.final_identity:
            raise WireError("semantic.replay_manifest", "final identity differs from replay tail")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "schema_version": REPLAY_FILE_SCHEMA_V8,
            "manifest": self.manifest.to_wire(),
            "steps": [step.to_wire() for step in self.steps],
            "final_identity": self.final_identity.to_wire(),
        }
