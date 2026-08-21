from __future__ import annotations

from dataclasses import dataclass

from .canonical import (
    parse_u64_number,
    parse_uint,
    require_digest,
    require_exact_keys,
    require_nonempty,
    uint_wire,
)
from .decision import DecisionResponse, DecisionResponseV2
from .episode import EpisodeStatus
from .errors import WireError
from .persistence import calculate_checkpoint_digest_v3

REPLAY_MANIFEST_SCHEMA = "replay-manifest.v1"
REPLAY_FILE_SCHEMA = "authoritative-replay.v1"
REPLAY_MANIFEST_SCHEMA_V2 = "replay-manifest.v2"
REPLAY_FILE_SCHEMA_V2 = "authoritative-replay.v2"
REPLAY_MANIFEST_SCHEMA_V3 = "replay-manifest.v3"
REPLAY_FILE_SCHEMA_V3 = "authoritative-replay.v3"
REPLAY_STEP_SCHEMA_V3 = "replay-step.v3"


@dataclass(frozen=True, slots=True)
class KernelIdentityV1:
    implementation_id: str
    semantic_version: str
    build_profile: str

    @classmethod
    def from_wire(cls, value: object) -> KernelIdentityV1:
        obj = require_exact_keys(value, {"implementation_id", "semantic_version", "build_profile"})
        return cls(
            *(
                require_nonempty(obj[key], key)
                for key in ("implementation_id", "semantic_version", "build_profile")
            )
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "build_profile": require_nonempty(self.build_profile, "build_profile"),
            "implementation_id": require_nonempty(self.implementation_id, "implementation_id"),
            "semantic_version": require_nonempty(self.semantic_version, "semantic_version"),
        }


@dataclass(frozen=True, slots=True)
class ReplaySchemaVersionsV1:
    observation: str
    information_state: str
    decision: str
    decision_response: str
    observed_event: str
    player_step: str
    replay_step: str

    @classmethod
    def from_wire(cls, value: object) -> ReplaySchemaVersionsV1:
        keys = {
            "observation",
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
                "observed_event",
                "player_step",
                "replay_step",
            )
        }


@dataclass(frozen=True, slots=True)
class RandomnessIdentityV1:
    algorithm_id: str
    derivation_version: str
    root_seed_hex: str

    @classmethod
    def from_wire(cls, value: object) -> RandomnessIdentityV1:
        obj = require_exact_keys(value, {"algorithm_id", "derivation_version", "root_seed_hex"})
        seed = obj["root_seed_hex"]
        if (
            not isinstance(seed, str)
            or len(seed) != 64
            or any(ch not in "0123456789abcdef" for ch in seed)
        ):
            raise WireError("semantic.replay_manifest", "root seed is not canonical hex")
        return cls(
            require_nonempty(obj["algorithm_id"], "algorithm_id"),
            require_nonempty(obj["derivation_version"], "derivation_version"),
            seed,
        )

    def to_wire(self) -> dict[str, object]:
        return RandomnessIdentityV1.from_wire(
            {
                "algorithm_id": self.algorithm_id,
                "derivation_version": self.derivation_version,
                "root_seed_hex": self.root_seed_hex,
            }
        )._raw()

    def _raw(self) -> dict[str, object]:
        return {
            "algorithm_id": self.algorithm_id,
            "derivation_version": self.derivation_version,
            "root_seed_hex": self.root_seed_hex,
        }


@dataclass(frozen=True, slots=True)
class DeckIdentityV1:
    player: int
    deck_id: str
    digest: str

    @classmethod
    def from_wire(cls, value: object) -> DeckIdentityV1:
        obj = require_exact_keys(value, {"player", "deck_id", "digest"})
        return cls(
            parse_uint(obj["player"]),
            obj["deck_id"],  # Allow empty for now; validated at manifest level
            require_digest(obj["digest"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "deck_id": require_nonempty(self.deck_id, "deck_id"),
            "digest": require_digest(self.digest),
            "player": uint_wire(self.player),
        }


@dataclass(frozen=True, slots=True)
class ReplayManifestV1:
    schema_version: str
    engine_build: str
    kernel: KernelIdentityV1
    rules_snapshot: str
    format_policy_snapshot: str
    oracle_snapshot: str
    card_bundle: str
    schemas: ReplaySchemaVersionsV1
    randomness: RandomnessIdentityV1
    decks: tuple[DeckIdentityV1, ...]
    initial_state_revision: int
    initial_state_digest: str

    @classmethod
    def from_wire(cls, value: object) -> ReplayManifestV1:
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
                "initial_state_revision",
                "initial_state_digest",
            },
        )
        if obj["schema_version"] != REPLAY_MANIFEST_SCHEMA or not isinstance(obj["decks"], list):
            raise WireError("decode.invalid_json", "unsupported replay manifest or deck list")
        result = cls(
            REPLAY_MANIFEST_SCHEMA,
            require_nonempty(obj["engine_build"], "engine_build"),
            KernelIdentityV1.from_wire(obj["kernel"]),
            require_nonempty(obj["rules_snapshot"], "rules_snapshot"),
            require_nonempty(obj["format_policy_snapshot"], "format_policy_snapshot"),
            require_nonempty(obj["oracle_snapshot"], "oracle_snapshot"),
            require_nonempty(obj["card_bundle"], "card_bundle"),
            ReplaySchemaVersionsV1.from_wire(obj["schemas"]),
            RandomnessIdentityV1.from_wire(obj["randomness"]),
            tuple(DeckIdentityV1.from_wire(item) for item in obj["decks"]),
            parse_uint(obj["initial_state_revision"]),
            require_digest(obj["initial_state_digest"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if not self.decks:
            raise WireError("semantic.replay_manifest", "at least one deck is required")
        players = [deck.player for deck in self.decks]
        if len(players) != len(set(players)):
            raise WireError("semantic.replay_manifest", "duplicate deck player")
        if any(deck.deck_id == "" for deck in self.decks):
            raise WireError("semantic.replay_manifest", "deck_id must not be empty")

    def to_wire(self) -> dict[str, object]:
        return {
            "card_bundle": require_nonempty(self.card_bundle, "card_bundle"),
            "decks": [deck.to_wire() for deck in self.decks],
            "engine_build": require_nonempty(self.engine_build, "engine_build"),
            "format_policy_snapshot": require_nonempty(
                self.format_policy_snapshot, "format_policy_snapshot"
            ),
            "initial_state_digest": require_digest(self.initial_state_digest),
            "initial_state_revision": uint_wire(self.initial_state_revision),
            "kernel": self.kernel.to_wire(),
            "oracle_snapshot": require_nonempty(self.oracle_snapshot, "oracle_snapshot"),
            "randomness": self.randomness.to_wire(),
            "rules_snapshot": require_nonempty(self.rules_snapshot, "rules_snapshot"),
            "schema_version": self.schema_version,
            "schemas": self.schemas.to_wire(),
        }


@dataclass(frozen=True, slots=True)
class ReplayStepV1:
    step_index: int
    state_revision_before: int
    response: DecisionResponse
    accepted: bool
    state_revision_after: int
    state_digest_after: str

    @classmethod
    def from_wire(cls, value: object) -> ReplayStepV1:
        obj = require_exact_keys(
            value,
            {
                "step_index",
                "state_revision_before",
                "response",
                "accepted",
                "state_revision_after",
                "state_digest_after",
            },
        )
        if not isinstance(obj["accepted"], bool):
            raise WireError("decode.invalid_json", "accepted must be boolean")
        return cls(
            parse_u64_number(obj["step_index"]),
            parse_uint(obj["state_revision_before"]),
            DecisionResponse.from_wire(obj["response"]),
            obj["accepted"],
            parse_uint(obj["state_revision_after"]),
            require_digest(obj["state_digest_after"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "accepted": self.accepted,
            "response": self.response.to_wire(),
            "state_digest_after": require_digest(self.state_digest_after),
            "state_revision_after": uint_wire(self.state_revision_after),
            "state_revision_before": uint_wire(self.state_revision_before),
            "step_index": parse_u64_number(self.step_index),
        }


@dataclass(frozen=True, slots=True)
class AuthoritativeReplayV1:
    schema_version: str
    manifest: ReplayManifestV1
    steps: tuple[ReplayStepV1, ...]
    final_state_revision: int
    final_state_digest: str

    @classmethod
    def from_wire(cls, value: object) -> AuthoritativeReplayV1:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "manifest",
                "steps",
                "final_state_revision",
                "final_state_digest",
            },
        )
        if obj["schema_version"] != REPLAY_FILE_SCHEMA or not isinstance(obj["steps"], list):
            raise WireError("decode.invalid_json", "unsupported replay or step list")
        result = cls(
            REPLAY_FILE_SCHEMA,
            ReplayManifestV1.from_wire(obj["manifest"]),
            tuple(ReplayStepV1.from_wire(item) for item in obj["steps"]),
            parse_uint(obj["final_state_revision"]),
            require_digest(obj["final_state_digest"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        revision = self.manifest.initial_state_revision
        state_digest = self.manifest.initial_state_digest
        for index, step in enumerate(self.steps):
            if (
                step.step_index != index
                or step.state_revision_before != revision
                or step.response.state_revision != revision
            ):
                raise WireError("semantic.replay", "replay revisions are discontinuous")
            if step.accepted and step.state_revision_after <= step.state_revision_before:
                raise WireError("semantic.replay", "accepted step did not advance revision")
            if not step.accepted and (
                step.state_revision_after != step.state_revision_before
                or step.state_digest_after != state_digest
            ):
                raise WireError(
                    "semantic.replay",
                    "rejected step changed revision or full-state identity",
                )
            revision = step.state_revision_after
            state_digest = step.state_digest_after
        if self.final_state_revision != revision:
            raise WireError("semantic.replay", "final revision differs")
        if self.steps:
            if self.final_state_digest != self.steps[-1].state_digest_after:
                raise WireError("semantic.replay", "final digest differs")
        elif (
            self.final_state_revision != self.manifest.initial_state_revision
            or self.final_state_digest != self.manifest.initial_state_digest
        ):
            raise WireError("semantic.replay", "empty replay does not preserve initial identity")

    def to_wire(self) -> dict[str, object]:
        return {
            "final_state_digest": require_digest(self.final_state_digest),
            "final_state_revision": uint_wire(self.final_state_revision),
            "manifest": self.manifest.to_wire(),
            "schema_version": self.schema_version,
            "steps": [step.to_wire() for step in self.steps],
        }


# === V2 replay types ===


@dataclass(frozen=True, slots=True)
class RandomnessIdentityV2:
    contract_id: str
    root_seed_hex: str

    @classmethod
    def from_wire(cls, value: object) -> RandomnessIdentityV2:
        obj = require_exact_keys(value, {"contract_id", "root_seed_hex"})
        return cls(
            require_nonempty(obj["contract_id"], "contract_id"),
            require_digest(obj["root_seed_hex"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "contract_id": require_nonempty(self.contract_id, "contract_id"),
            "root_seed_hex": require_digest(self.root_seed_hex),
        }


@dataclass(frozen=True, slots=True)
class ReplayManifestV2:
    schema_version: str
    engine_build: str
    kernel: KernelIdentityV1
    rules_snapshot: str
    format_policy_snapshot: str
    oracle_snapshot: str
    card_bundle: str
    schemas: ReplaySchemaVersionsV1
    randomness: RandomnessIdentityV2
    decks: tuple[DeckIdentityV1, ...]
    initial_state_revision: int
    initial_state_digest: str

    @classmethod
    def from_wire(cls, value: object) -> ReplayManifestV2:
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
                "initial_state_revision",
                "initial_state_digest",
            },
        )
        if obj["schema_version"] != REPLAY_MANIFEST_SCHEMA_V2 or not isinstance(obj["decks"], list):
            raise WireError("decode.invalid_json", "unsupported replay manifest or deck list")
        result = cls(
            REPLAY_MANIFEST_SCHEMA_V2,
            require_nonempty(obj["engine_build"], "engine_build"),
            KernelIdentityV1.from_wire(obj["kernel"]),
            require_nonempty(obj["rules_snapshot"], "rules_snapshot"),
            require_nonempty(obj["format_policy_snapshot"], "format_policy_snapshot"),
            require_nonempty(obj["oracle_snapshot"], "oracle_snapshot"),
            require_nonempty(obj["card_bundle"], "card_bundle"),
            ReplaySchemaVersionsV1.from_wire(obj["schemas"]),
            RandomnessIdentityV2.from_wire(obj["randomness"]),
            tuple(DeckIdentityV1.from_wire(item) for item in obj["decks"]),
            parse_uint(obj["initial_state_revision"]),
            require_digest(obj["initial_state_digest"]),
        )
        result.validate()
        return result

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "card_bundle": require_nonempty(self.card_bundle, "card_bundle"),
            "decks": [deck.to_wire() for deck in self.decks],
            "engine_build": require_nonempty(self.engine_build, "engine_build"),
            "format_policy_snapshot": require_nonempty(
                self.format_policy_snapshot, "format_policy_snapshot"
            ),
            "initial_state_digest": require_digest(self.initial_state_digest),
            "initial_state_revision": uint_wire(self.initial_state_revision),
            "kernel": self.kernel.to_wire(),
            "oracle_snapshot": require_nonempty(self.oracle_snapshot, "oracle_snapshot"),
            "randomness": self.randomness.to_wire(),
            "rules_snapshot": require_nonempty(self.rules_snapshot, "rules_snapshot"),
            "schema_version": REPLAY_MANIFEST_SCHEMA_V2,
            "schemas": self.schemas.to_wire(),
        }

    def validate(self) -> None:
        if self.randomness.contract_id != "mtgml.rng.v1":
            raise WireError("semantic.replay_manifest", "unsupported RNG contract in replay")
        if not self.decks:
            raise WireError("semantic.replay_manifest", "decks must not be empty")
        seen_players = set()
        for deck in self.decks:
            if deck.player in seen_players:
                raise WireError("semantic.replay_manifest", "duplicate player in decks")
            seen_players.add(deck.player)
            if not deck.deck_id:
                raise WireError("semantic.replay_manifest", "deck_id must not be empty")
        if self.schemas.replay_step != "replay-step.v2":
            raise WireError("semantic.replay_manifest", "replay_step must be replay-step.v2")


@dataclass(frozen=True, slots=True)
class ReplayStepV2:
    step_index: int
    state_revision_before: int
    response: DecisionResponse
    accepted: bool
    state_revision_after: int
    state_digest_after: str

    @classmethod
    def from_wire(cls, value: object) -> ReplayStepV2:
        obj = require_exact_keys(
            value,
            {
                "step_index",
                "state_revision_before",
                "response",
                "accepted",
                "state_revision_after",
                "state_digest_after",
            },
        )
        if not isinstance(obj["accepted"], bool):
            raise WireError("decode.invalid_json", "accepted must be boolean")
        return cls(
            parse_u64_number(obj["step_index"]),
            parse_uint(obj["state_revision_before"]),
            DecisionResponse.from_wire(obj["response"]),
            obj["accepted"],
            parse_uint(obj["state_revision_after"]),
            require_digest(obj["state_digest_after"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "accepted": self.accepted,
            "response": self.response.to_wire(),
            "state_digest_after": require_digest(self.state_digest_after),
            "state_revision_after": uint_wire(self.state_revision_after),
            "state_revision_before": uint_wire(self.state_revision_before),
            "step_index": parse_u64_number(self.step_index),
        }


@dataclass(frozen=True, slots=True)
class AuthoritativeReplayV2:
    schema_version: str
    manifest: ReplayManifestV2
    steps: tuple[ReplayStepV2, ...]
    final_state_revision: int
    final_state_digest: str

    @classmethod
    def from_wire(cls, value: object) -> AuthoritativeReplayV2:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "manifest",
                "steps",
                "final_state_revision",
                "final_state_digest",
            },
        )
        if obj["schema_version"] != REPLAY_FILE_SCHEMA_V2 or not isinstance(obj["steps"], list):
            raise WireError("decode.invalid_json", "unsupported replay or step list")
        result = cls(
            REPLAY_FILE_SCHEMA_V2,
            ReplayManifestV2.from_wire(obj["manifest"]),
            tuple(ReplayStepV2.from_wire(item) for item in obj["steps"]),
            parse_uint(obj["final_state_revision"]),
            require_digest(obj["final_state_digest"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if self.schema_version != REPLAY_FILE_SCHEMA_V2:
            raise WireError("semantic.replay", "unsupported replay schema version")
        self.manifest.validate()
        revision = self.manifest.initial_state_revision
        state_digest = self.manifest.initial_state_digest
        for index, step in enumerate(self.steps):
            if (
                step.step_index != index
                or step.state_revision_before != revision
                or step.response.state_revision != revision
            ):
                raise WireError("semantic.replay", "replay revisions are discontinuous")
            if step.accepted and step.state_revision_after <= step.state_revision_before:
                raise WireError("semantic.replay", "accepted step did not advance revision")
            if not step.accepted and (
                step.state_revision_after != step.state_revision_before
                or step.state_digest_after != state_digest
            ):
                raise WireError(
                    "semantic.replay",
                    "rejected step changed revision or full-state identity",
                )
            revision = step.state_revision_after
            state_digest = step.state_digest_after
        if self.final_state_revision != revision:
            raise WireError("semantic.replay", "final revision differs")
        if self.steps:
            if self.final_state_digest != self.steps[-1].state_digest_after:
                raise WireError("semantic.replay", "final digest differs")
        elif (
            self.final_state_revision != self.manifest.initial_state_revision
            or self.final_state_digest != self.manifest.initial_state_digest
        ):
            raise WireError("semantic.replay", "empty replay does not preserve initial identity")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "final_state_digest": require_digest(self.final_state_digest),
            "final_state_revision": uint_wire(self.final_state_revision),
            "manifest": self.manifest.to_wire(),
            "schema_version": REPLAY_FILE_SCHEMA_V2,
            "steps": [step.to_wire() for step in self.steps],
        }


@dataclass(frozen=True, slots=True)
class EnvironmentLimitCountersV3:
    decisions_submitted: int
    accepted_transitions: int
    rule_events_emitted: int
    resource_units_consumed: int
    wall_clock_elapsed_millis: int

    @classmethod
    def from_wire(cls, value: object) -> EnvironmentLimitCountersV3:
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
        values = {
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
class CheckpointCodecIdentityV3:
    codec_id: str
    semantic_version: str

    @classmethod
    def from_wire(cls, value: object) -> CheckpointCodecIdentityV3:
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
class InitialEnvironmentIdentityV3:
    state_revision: int
    full_state_digest: str
    episode_status: EpisodeStatus
    environment_limit_counters: EnvironmentLimitCountersV3
    checkpoint_codec_identity: CheckpointCodecIdentityV3
    checkpoint_digest: str

    @classmethod
    def from_wire(cls, value: object) -> InitialEnvironmentIdentityV3:
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
            EnvironmentLimitCountersV3.from_wire(obj["environment_limit_counters"]),
            CheckpointCodecIdentityV3.from_wire(obj["checkpoint_codec_identity"]),
            require_digest(obj["checkpoint_digest"]),
        )
        return result

    def validate(self, *, error_code: str = "semantic.replay") -> None:
        expected = calculate_checkpoint_digest_v3(
            self.full_state_digest,
            self.episode_status,
            self.environment_limit_counters.as_dict(),
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


@dataclass(frozen=True, slots=True)
class ReplayManifestV3:
    schema_version: str
    engine_build: str
    kernel: KernelIdentityV1
    rules_snapshot: str
    format_policy_snapshot: str
    oracle_snapshot: str
    card_bundle: str
    schemas: ReplaySchemaVersionsV1
    randomness: RandomnessIdentityV2
    decks: tuple[DeckIdentityV1, ...]
    initial_identity: InitialEnvironmentIdentityV3

    @classmethod
    def from_wire(cls, value: object) -> ReplayManifestV3:
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
        if obj["schema_version"] != REPLAY_MANIFEST_SCHEMA_V3 or not isinstance(obj["decks"], list):
            raise WireError("decode.invalid_json", "unsupported replay manifest V3")
        result = cls(
            REPLAY_MANIFEST_SCHEMA_V3,
            require_nonempty(obj["engine_build"], "engine_build"),
            KernelIdentityV1.from_wire(obj["kernel"]),
            require_nonempty(obj["rules_snapshot"], "rules_snapshot"),
            require_nonempty(obj["format_policy_snapshot"], "format_policy_snapshot"),
            require_nonempty(obj["oracle_snapshot"], "oracle_snapshot"),
            require_nonempty(obj["card_bundle"], "card_bundle"),
            ReplaySchemaVersionsV1.from_wire(obj["schemas"]),
            RandomnessIdentityV2.from_wire(obj["randomness"]),
            tuple(DeckIdentityV1.from_wire(item) for item in obj["decks"]),
            InitialEnvironmentIdentityV3.from_wire(obj["initial_identity"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if self.randomness.contract_id != "mtgml.rng.v1":
            raise WireError("semantic.replay_manifest", "unsupported RNG contract")
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
        if self.schemas.replay_step != REPLAY_STEP_SCHEMA_V3:
            raise WireError("semantic.replay_manifest", "replay-step schema is not V3")
        if not self.decks:
            raise WireError("semantic.replay_manifest", "decks must not be empty")
        players = [deck.player for deck in self.decks]
        if len(players) != len(set(players)) or any(not deck.deck_id for deck in self.decks):
            raise WireError("semantic.replay_manifest", "deck identities are not unique")
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
            "schema_version": REPLAY_MANIFEST_SCHEMA_V3,
            "schemas": self.schemas.to_wire(),
        }


@dataclass(frozen=True, slots=True)
class ReplayStepV3:
    step_index: int
    actor: int
    checkpoint_digest_before: str
    state_revision_before: int
    response: DecisionResponseV2
    accepted: bool
    state_revision_after: int
    full_state_digest_after: str
    episode_status_after: EpisodeStatus
    environment_limit_counters_after: EnvironmentLimitCountersV3
    checkpoint_digest_after: str

    @classmethod
    def from_wire(cls, value: object) -> ReplayStepV3:
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
            EnvironmentLimitCountersV3.from_wire(obj["environment_limit_counters_after"]),
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
class AuthoritativeReplayV3:
    schema_version: str
    manifest: ReplayManifestV3
    steps: tuple[ReplayStepV3, ...]
    final_identity: InitialEnvironmentIdentityV3

    @classmethod
    def from_wire(cls, value: object) -> AuthoritativeReplayV3:
        obj = require_exact_keys(value, {"schema_version", "manifest", "steps", "final_identity"})
        if obj["schema_version"] != REPLAY_FILE_SCHEMA_V3 or not isinstance(obj["steps"], list):
            raise WireError("decode.invalid_json", "unsupported authoritative replay V3")
        try:
            manifest = ReplayManifestV3.from_wire(obj["manifest"])
        except WireError as exc:
            raise WireError("semantic.replay", exc.message) from exc
        result = cls(
            REPLAY_FILE_SCHEMA_V3,
            manifest,
            tuple(ReplayStepV3.from_wire(item) for item in obj["steps"]),
            InitialEnvironmentIdentityV3.from_wire(obj["final_identity"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        self.manifest.validate()
        previous = self.manifest.initial_identity
        for index, step in enumerate(self.steps):
            if (
                step.step_index != index
                or step.actor == 0
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
            elif step.state_revision_after <= previous.state_revision:
                raise WireError("semantic.replay", "accepted step did not advance revision")
            previous = InitialEnvironmentIdentityV3(
                step.state_revision_after,
                step.full_state_digest_after,
                step.episode_status_after,
                step.environment_limit_counters_after,
                previous.checkpoint_codec_identity,
                step.checkpoint_digest_after,
            )
            previous.validate(error_code="semantic.replay")
        if self.final_identity != previous:
            raise WireError("semantic.replay", "final identity differs")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "final_identity": self.final_identity.to_wire(),
            "manifest": self.manifest.to_wire(),
            "schema_version": REPLAY_FILE_SCHEMA_V3,
            "steps": [step.to_wire() for step in self.steps],
        }
