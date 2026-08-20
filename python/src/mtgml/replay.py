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
from .decision import DecisionResponse
from .errors import WireError

REPLAY_MANIFEST_SCHEMA = "replay-manifest.v1"
REPLAY_FILE_SCHEMA = "authoritative-replay.v1"
REPLAY_MANIFEST_SCHEMA_V2 = "replay-manifest.v2"
REPLAY_FILE_SCHEMA_V2 = "authoritative-replay.v2"


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
