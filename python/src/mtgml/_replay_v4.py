from __future__ import annotations

from dataclasses import dataclass
from itertools import pairwise

from ._replay_common import DeckIdentityV1, KernelIdentityV1
from ._replay_v2 import RandomnessIdentityV2
from .canonical import (
    parse_u64_number,
    parse_uint,
    require_digest,
    require_exact_keys,
    require_nonempty,
    uint_wire,
)
from .decision import DecisionResponseV2
from .episode import EpisodeStatus
from .errors import WireError
from .persistence import calculate_checkpoint_digest_v4

REPLAY_MANIFEST_SCHEMA_V4 = "replay-manifest.v4"
REPLAY_FILE_SCHEMA_V4 = "authoritative-replay.v4"
REPLAY_STEP_SCHEMA_V4 = "replay-step.v4"

CHECKPOINT_CODEC_ID_V4 = "in-memory-reference"
CHECKPOINT_CODEC_VERSION_V4 = "4"


@dataclass(frozen=True, slots=True)
class ReplaySchemaVersionsV4:
    observation: str
    observation_payload_codec: str
    information_state: str
    decision: str
    decision_response: str
    observed_event: str
    player_step: str
    replay_step: str

    @classmethod
    def from_wire(cls, value: object) -> ReplaySchemaVersionsV4:
        keys = {
            "observation",
            "observation_payload_codec",
            "information_state",
            "decision",
            "decision_response",
            "observed_event",
            "player_step",
            "replay_step",
        }
        obj = require_exact_keys(value, keys)
        return cls(**{key: require_nonempty(obj[key], key) for key in keys})

    def to_wire(self) -> dict[str, object]:
        return {
            key: require_nonempty(getattr(self, key), key)
            for key in (
                "decision",
                "decision_response",
                "information_state",
                "observation",
                "observation_payload_codec",
                "observed_event",
                "player_step",
                "replay_step",
            )
        }


@dataclass(frozen=True, slots=True)
class EnvironmentLimitCountersV4:
    decisions_submitted: int
    accepted_transitions: int
    rule_events_emitted: int
    resource_units_consumed: int
    wall_clock_elapsed_millis: int

    @classmethod
    def from_wire(cls, value: object) -> EnvironmentLimitCountersV4:
        obj = require_exact_keys(
            value,
            {
                "decisions_submitted",
                "accepted_transitions",
                "rule_events_emitted",
                "resource_units_consumed",
                "wall_clock_elapsed_millis",
            },
        )
        return cls(
            parse_u64_number(obj["decisions_submitted"]),
            parse_u64_number(obj["accepted_transitions"]),
            parse_u64_number(obj["rule_events_emitted"]),
            parse_u64_number(obj["resource_units_consumed"]),
            parse_u64_number(obj["wall_clock_elapsed_millis"]),
        )

    def to_wire(self) -> dict[str, object]:
        values: dict[str, object] = {
            "decisions_submitted": self.decisions_submitted,
            "accepted_transitions": self.accepted_transitions,
            "rule_events_emitted": self.rule_events_emitted,
            "resource_units_consumed": self.resource_units_consumed,
            "wall_clock_elapsed_millis": self.wall_clock_elapsed_millis,
        }
        for name, value in values.items():
            if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 2**64 - 1:
                raise WireError("encode.serialization", f"{name} is outside u64")
        return values

    def as_dict(self) -> dict[str, int]:
        return {
            "decisions_submitted": self.decisions_submitted,
            "accepted_transitions": self.accepted_transitions,
            "rule_events_emitted": self.rule_events_emitted,
            "resource_units_consumed": self.resource_units_consumed,
            "wall_clock_elapsed_millis": self.wall_clock_elapsed_millis,
        }


@dataclass(frozen=True, slots=True)
class CheckpointCodecIdentityV4:
    codec_id: str
    semantic_version: str

    @classmethod
    def from_wire(cls, value: object) -> CheckpointCodecIdentityV4:
        obj = require_exact_keys(value, {"codec_id", "semantic_version"})
        return cls(
            require_nonempty(obj["codec_id"], "codec_id"),
            require_nonempty(obj["semantic_version"], "semantic_version"),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "codec_id": require_nonempty(self.codec_id, "codec_id"),
            "semantic_version": require_nonempty(self.semantic_version, "semantic_version"),
        }


@dataclass(frozen=True, slots=True)
class InitialEnvironmentIdentityV4:
    state_revision: int
    full_state_digest: str
    episode_status: EpisodeStatus
    environment_limit_counters: EnvironmentLimitCountersV4
    checkpoint_codec_identity: CheckpointCodecIdentityV4
    checkpoint_digest: str

    @classmethod
    def from_wire(cls, value: object) -> InitialEnvironmentIdentityV4:
        obj = require_exact_keys(
            value,
            {
                "state_revision",
                "full_state_digest",
                "episode_status",
                "environment_limit_counters",
                "checkpoint_codec_identity",
                "checkpoint_digest",
            },
        )
        result = cls(
            parse_uint(obj["state_revision"]),
            require_digest(obj["full_state_digest"]),
            EpisodeStatus.from_wire(obj["episode_status"]),
            EnvironmentLimitCountersV4.from_wire(obj["environment_limit_counters"]),
            CheckpointCodecIdentityV4.from_wire(obj["checkpoint_codec_identity"]),
            require_digest(obj["checkpoint_digest"]),
        )
        return result

    def validate(self, *, error_code: str = "semantic.replay") -> None:
        if (
            self.checkpoint_codec_identity.codec_id != CHECKPOINT_CODEC_ID_V4
            or self.checkpoint_codec_identity.semantic_version != CHECKPOINT_CODEC_VERSION_V4
        ):
            raise WireError(error_code, "checkpoint codec identity is not V4")
        counters = self.environment_limit_counters
        if counters.accepted_transitions > counters.decisions_submitted:
            raise WireError(error_code, "accepted transitions exceed submitted decisions")
        expected = calculate_checkpoint_digest_v4(
            self.full_state_digest,
            self.episode_status,
            counters.as_dict(),
            self.checkpoint_codec_identity.codec_id,
            self.checkpoint_codec_identity.semantic_version,
        )
        if self.checkpoint_digest != expected:
            raise WireError(error_code, "checkpoint identity does not match")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "checkpoint_codec_identity": self.checkpoint_codec_identity.to_wire(),
            "checkpoint_digest": require_digest(self.checkpoint_digest),
            "environment_limit_counters": self.environment_limit_counters.to_wire(),
            "episode_status": self.episode_status.to_wire(),
            "full_state_digest": require_digest(self.full_state_digest),
            "state_revision": uint_wire(self.state_revision),
        }


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


@dataclass(frozen=True, slots=True)
class ReplayManifestV4:
    schema_version: str
    engine_build: str
    kernel: KernelIdentityV1
    rules_snapshot: str
    format_policy_snapshot: str
    oracle_snapshot: str
    card_bundle: str
    schemas: ReplaySchemaVersionsV4
    randomness: RandomnessIdentityV2
    decks: tuple[DeckIdentityV1, ...]
    initial_identity: InitialEnvironmentIdentityV4

    @classmethod
    def from_wire(cls, value: object) -> ReplayManifestV4:
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
            },
        )
        if not isinstance(obj["schema_version"], str) or not isinstance(obj["decks"], list):
            raise WireError("decode.invalid_json", "unsupported replay manifest V4")
        result = cls(
            str(obj["schema_version"]),
            require_nonempty(obj["engine_build"], "engine_build"),
            KernelIdentityV1.from_wire(obj["kernel"]),
            require_nonempty(obj["rules_snapshot"], "rules_snapshot"),
            require_nonempty(obj["format_policy_snapshot"], "format_policy_snapshot"),
            require_nonempty(obj["oracle_snapshot"], "oracle_snapshot"),
            require_nonempty(obj["card_bundle"], "card_bundle"),
            ReplaySchemaVersionsV4.from_wire(obj["schemas"]),
            RandomnessIdentityV2.from_wire(obj["randomness"]),
            tuple(DeckIdentityV1.from_wire(item) for item in obj["decks"]),
            InitialEnvironmentIdentityV4.from_wire(obj["initial_identity"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if self.schema_version != REPLAY_MANIFEST_SCHEMA_V4:
            raise WireError("semantic.replay_manifest", "unsupported replay manifest V4")
        for label, field in (
            ("engine_build", self.engine_build),
            ("rules_snapshot", self.rules_snapshot),
            ("format_policy_snapshot", self.format_policy_snapshot),
            ("oracle_snapshot", self.oracle_snapshot),
            ("card_bundle", self.card_bundle),
        ):
            if not field:
                raise WireError("semantic.replay_manifest", f"{label} must not be empty")
        if self.randomness.contract_id != "mtgml.rng.v1":
            raise WireError("semantic.replay_manifest", "unsupported RNG contract")
        if self.schemas.observation != "observation-envelope.v1":
            raise WireError("semantic.replay_manifest", "observation schema is not V1")
        if self.schemas.observation_payload_codec != "synthetic-m3-observation.v1":
            raise WireError("semantic.replay_manifest", "observation payload codec is not M3")
        if self.schemas.decision != "player-decision-request.v2":
            raise WireError("semantic.replay_manifest", "decision schema is not V2")
        if self.schemas.decision_response != "decision-response.v2":
            raise WireError("semantic.replay_manifest", "decision response schema is not V2")
        if self.schemas.information_state != "information-state-envelope.v2":
            raise WireError("semantic.replay_manifest", "information-state schema is not V2")
        if self.schemas.observed_event != "observed-event-envelope.v2":
            raise WireError("semantic.replay_manifest", "observed-event schema is not V2")
        if self.schemas.player_step != "player-step.v2":
            raise WireError("semantic.replay_manifest", "player-step schema is not V2")
        if self.schemas.replay_step != REPLAY_STEP_SCHEMA_V4:
            raise WireError("semantic.replay_manifest", "replay-step schema is not V4")
        if not self.decks:
            raise WireError("semantic.replay_manifest", "decks must not be empty")
        players: list[int] = []
        seen: set[int] = set()
        previous: int | None = None
        for deck in self.decks:
            if not deck.deck_id or deck.player in seen:
                raise WireError("semantic.replay_manifest", "deck identities are not unique")
            seen.add(deck.player)
            if previous is not None and previous > deck.player:
                raise WireError("semantic.replay_manifest", "decks are not in canonical order")
            previous = deck.player
            players.append(deck.player)
        _validate_status_for_players(self.initial_identity.episode_status, seen)
        self.initial_identity.validate(error_code="semantic.replay_manifest")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "card_bundle": require_nonempty(self.card_bundle, "card_bundle"),
            "decks": [deck.to_wire() for deck in self.decks],
            "engine_build": require_nonempty(self.engine_build, "engine_build"),
            "format_policy_snapshot": require_nonempty(
                self.format_policy_snapshot, "format_policy_snapshot"
            ),
            "initial_identity": self.initial_identity.to_wire(),
            "kernel": self.kernel.to_wire(),
            "oracle_snapshot": require_nonempty(self.oracle_snapshot, "oracle_snapshot"),
            "randomness": self.randomness.to_wire(),
            "rules_snapshot": require_nonempty(self.rules_snapshot, "rules_snapshot"),
            "schema_version": REPLAY_MANIFEST_SCHEMA_V4,
            "schemas": self.schemas.to_wire(),
        }


@dataclass(frozen=True, slots=True)
class ReplayStepV4:
    step_index: int
    actor: int
    checkpoint_digest_before: str
    state_revision_before: int
    response: DecisionResponseV2
    accepted: bool
    state_revision_after: int
    full_state_digest_after: str
    episode_status_after: EpisodeStatus
    environment_limit_counters_after: EnvironmentLimitCountersV4
    checkpoint_digest_after: str

    @classmethod
    def from_wire(cls, value: object) -> ReplayStepV4:
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
            DecisionResponseV2.from_wire(obj["response"]),
            obj["accepted"],
            parse_uint(obj["state_revision_after"]),
            require_digest(obj["full_state_digest_after"]),
            EpisodeStatus.from_wire(obj["episode_status_after"]),
            EnvironmentLimitCountersV4.from_wire(obj["environment_limit_counters_after"]),
            require_digest(obj["checkpoint_digest_after"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "accepted": self.accepted,
            "actor": uint_wire(self.actor),
            "checkpoint_digest_after": require_digest(self.checkpoint_digest_after),
            "checkpoint_digest_before": require_digest(self.checkpoint_digest_before),
            "environment_limit_counters_after": self.environment_limit_counters_after.to_wire(),
            "episode_status_after": self.episode_status_after.to_wire(),
            "full_state_digest_after": require_digest(self.full_state_digest_after),
            "response": self.response.to_wire(),
            "state_revision_after": uint_wire(self.state_revision_after),
            "state_revision_before": uint_wire(self.state_revision_before),
            "step_index": parse_u64_number(self.step_index),
        }


@dataclass(frozen=True, slots=True)
class AuthoritativeReplayV4:
    schema_version: str
    manifest: ReplayManifestV4
    steps: tuple[ReplayStepV4, ...]
    final_identity: InitialEnvironmentIdentityV4

    @classmethod
    def from_wire(cls, value: object) -> AuthoritativeReplayV4:
        obj = require_exact_keys(value, {"schema_version", "manifest", "steps", "final_identity"})
        if not isinstance(obj["schema_version"], str) or not isinstance(obj["steps"], list):
            raise WireError("decode.invalid_json", "unsupported authoritative replay V4")
        try:
            manifest = ReplayManifestV4.from_wire(obj["manifest"])
        except WireError as exc:
            raise WireError("semantic.replay", exc.message) from exc
        result = cls(
            str(obj["schema_version"]),
            manifest,
            tuple(ReplayStepV4.from_wire(item) for item in obj["steps"]),
            InitialEnvironmentIdentityV4.from_wire(obj["final_identity"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if self.schema_version != REPLAY_FILE_SCHEMA_V4:
            raise WireError("semantic.replay", "unsupported authoritative replay V4")
        self.manifest.validate()
        previous = self.manifest.initial_identity
        manifest_players = {deck.player for deck in self.manifest.decks}
        for index, step in enumerate(self.steps):
            if (
                step.step_index != index
                or step.checkpoint_digest_before != previous.checkpoint_digest
                or step.state_revision_before != previous.state_revision
                or step.response.state_revision != previous.state_revision
            ):
                raise WireError("semantic.replay", "replay identity is discontinuous")
            step.response.validate()
            if not step.accepted:
                if (
                    step.state_revision_after != previous.state_revision
                    or step.full_state_digest_after != previous.full_state_digest
                    or step.episode_status_after != previous.episode_status
                    or step.environment_limit_counters_after != previous.environment_limit_counters
                    or step.checkpoint_digest_after != previous.checkpoint_digest
                ):
                    raise WireError("semantic.replay", "rejected step mutated identity")
            else:
                if step.state_revision_after != previous.state_revision + 1:
                    raise WireError(
                        "semantic.replay", "accepted step did not advance revision contiguously"
                    )
                after = step.environment_limit_counters_after
                before = previous.environment_limit_counters
                if (
                    after.decisions_submitted != before.decisions_submitted + 1
                    or after.accepted_transitions != before.accepted_transitions + 1
                    or after.accepted_transitions > after.decisions_submitted
                    or after.rule_events_emitted < before.rule_events_emitted
                    or after.resource_units_consumed < before.resource_units_consumed
                    or after.wall_clock_elapsed_millis < before.wall_clock_elapsed_millis
                ):
                    raise WireError(
                        "semantic.replay",
                        "accepted step counter progression is not deterministic",
                    )
            previous = InitialEnvironmentIdentityV4(
                step.state_revision_after,
                step.full_state_digest_after,
                step.episode_status_after,
                step.environment_limit_counters_after,
                previous.checkpoint_codec_identity,
                step.checkpoint_digest_after,
            )
            try:
                previous.validate(error_code="semantic.replay")
            except WireError as exc:
                raise WireError("semantic.replay", exc.message) from exc
            if previous.episode_status.kind != "running":
                ordered = [o.player for o in previous.episode_status.players]
                if any(left >= right for left, right in pairwise(ordered)):
                    raise WireError("semantic.replay", "status players are not canonical")
                if {o.player for o in previous.episode_status.players} != manifest_players:
                    raise WireError("semantic.replay", "status universe differs")
        if self.final_identity != previous:
            raise WireError("semantic.replay", "final identity differs")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "final_identity": self.final_identity.to_wire(),
            "manifest": self.manifest.to_wire(),
            "schema_version": REPLAY_FILE_SCHEMA_V4,
            "steps": [step.to_wire() for step in self.steps],
        }
