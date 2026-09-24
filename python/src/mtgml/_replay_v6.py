from __future__ import annotations

from dataclasses import dataclass
from itertools import pairwise

from ._replay_common import DeckIdentityV1, KernelIdentityV1
from ._replay_v2 import RandomnessIdentityV2
from ._replay_v4 import (
    CheckpointCodecIdentityV4,
    EnvironmentLimitCountersV4,
)
from ._replay_v5 import ExecutionIdentityV1, SemanticContractMaterialV5
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
from .persistence import (
    CHECKPOINT_CODEC_ID_V6,
    CHECKPOINT_CODEC_VERSION_V6,
    calculate_checkpoint_digest_v6,
)

SYNTHETIC_OBSERVATION_CODEC = "synthetic-m3-observation.v1"
MAGIC_OBSERVATION_CODEC = "magic-m3-observation.v1"
COMBAT_OBSERVATION_CODEC = "magic-combat-observation.v2"
COMBAT_BLOCKERS_OBSERVATION_CODEC = "magic-combat-observation.v3"
SBA_CAPABILITY_KEY = "rules/state-based-actions-combat"
SBA_CAPABILITY_VERSION = "0.1.0"

REPLAY_MANIFEST_SCHEMA_V6 = "replay-manifest.v6"
REPLAY_FILE_SCHEMA_V6 = "authoritative-replay.v6"
REPLAY_STEP_SCHEMA_V6 = "replay-step.v6"

__all__ = [
    "CHECKPOINT_CODEC_ID_V6",
    "CHECKPOINT_CODEC_VERSION_V6",
    "REPLAY_FILE_SCHEMA_V6",
    "REPLAY_MANIFEST_SCHEMA_V6",
    "REPLAY_STEP_SCHEMA_V6",
    "AuthoritativeReplayV6",
    "ExecutionIdentityV1",
    "InitialEnvironmentIdentityV6",
    "ReplayManifestV6",
    "ReplaySchemaVersionsV6",
    "ReplayStepV6",
]


@dataclass(frozen=True, slots=True)
class ReplaySchemaVersionsV6:
    observation: str
    observation_payload_codec: str
    information_state: str
    decision: str
    decision_response: str
    observed_event: str
    player_step: str
    replay_step: str

    @classmethod
    def from_wire(cls, value: object) -> ReplaySchemaVersionsV6:
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
class InitialEnvironmentIdentityV6:
    state_revision: int
    full_state_digest: str
    episode_status: EpisodeStatus
    environment_limit_counters: EnvironmentLimitCountersV4
    checkpoint_codec_identity: CheckpointCodecIdentityV4
    checkpoint_digest: str
    execution_identity: ExecutionIdentityV1

    @classmethod
    def from_wire(cls, value: object) -> InitialEnvironmentIdentityV6:
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
        return cls(
            parse_uint(obj["state_revision"]),
            require_digest(obj["full_state_digest"]),
            EpisodeStatus.from_wire(obj["episode_status"]),
            EnvironmentLimitCountersV4.from_wire(obj["environment_limit_counters"]),
            CheckpointCodecIdentityV4.from_wire(obj["checkpoint_codec_identity"]),
            require_digest(obj["checkpoint_digest"]),
            ExecutionIdentityV1.from_wire(obj["execution_identity"]),
        )

    def recompute_checkpoint_digest(self) -> str:
        counters = self.environment_limit_counters.as_dict()
        return calculate_checkpoint_digest_v6(
            full_state_digest=self.full_state_digest,
            status=self.episode_status,
            counters=counters,
            codec_id=self.checkpoint_codec_identity.codec_id,
            semantic_version=self.checkpoint_codec_identity.semantic_version,
            program_kind=self.execution_identity.program_kind,
            semantic_contract_id=self.execution_identity.semantic_contract_id,
        )

    def validate(self, *, error_code: str = "semantic.replay") -> None:
        if (
            self.checkpoint_codec_identity.codec_id != CHECKPOINT_CODEC_ID_V6
            or self.checkpoint_codec_identity.semantic_version != CHECKPOINT_CODEC_VERSION_V6
        ):
            raise WireError(error_code, "checkpoint codec identity is not V6")
        counters = self.environment_limit_counters
        if counters.accepted_transitions > counters.decisions_submitted:
            raise WireError(error_code, "accepted transitions exceed submitted decisions")
        expected = self.recompute_checkpoint_digest()
        if self.checkpoint_digest != expected:
            raise WireError(error_code, "checkpoint identity does not match")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "checkpoint_codec_identity": self.checkpoint_codec_identity.to_wire(),
            "checkpoint_digest": require_digest(self.checkpoint_digest),
            "environment_limit_counters": self.environment_limit_counters.to_wire(),
            "episode_status": self.episode_status.to_wire(),
            "execution_identity": self.execution_identity.to_wire(),
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
class ReplayManifestV6:
    schema_version: str
    engine_build: str
    kernel: KernelIdentityV1
    rules_snapshot: str
    format_policy_snapshot: str
    oracle_snapshot: str
    card_bundle: str
    schemas: ReplaySchemaVersionsV6
    randomness: RandomnessIdentityV2
    decks: tuple[DeckIdentityV1, ...]
    initial_identity: InitialEnvironmentIdentityV6
    execution_identity: ExecutionIdentityV1
    semantic_contract: SemanticContractMaterialV5

    @classmethod
    def from_wire(cls, value: object) -> ReplayManifestV6:
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
        if not isinstance(obj["schema_version"], str) or not isinstance(obj["decks"], list):
            raise WireError("decode.invalid_json", "unsupported replay manifest V6")
        result = cls(
            str(obj["schema_version"]),
            require_nonempty(obj["engine_build"], "engine_build"),
            KernelIdentityV1.from_wire(obj["kernel"]),
            require_nonempty(obj["rules_snapshot"], "rules_snapshot"),
            require_nonempty(obj["format_policy_snapshot"], "format_policy_snapshot"),
            require_nonempty(obj["oracle_snapshot"], "oracle_snapshot"),
            require_nonempty(obj["card_bundle"], "card_bundle"),
            ReplaySchemaVersionsV6.from_wire(obj["schemas"]),
            RandomnessIdentityV2.from_wire(obj["randomness"]),
            tuple(DeckIdentityV1.from_wire(item) for item in obj["decks"]),
            InitialEnvironmentIdentityV6.from_wire(obj["initial_identity"]),
            ExecutionIdentityV1.from_wire(obj["execution_identity"]),
            SemanticContractMaterialV5.from_wire(obj["semantic_contract"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if self.schema_version != REPLAY_MANIFEST_SCHEMA_V6:
            raise WireError("semantic.replay_manifest", "unsupported replay manifest V6")
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
        rules_manifest = self.semantic_contract.rules_manifest
        rules_authority = rules_manifest.get("rules_authority")
        closure = rules_manifest.get("capability_closure")
        magic_semantics_admitted = (
            isinstance(rules_authority, dict)
            and rules_authority.get("variant") == "comprehensive_rules"
            and isinstance(closure, list)
            and any(
                isinstance(item, dict)
                and item.get("key") == SBA_CAPABILITY_KEY
                and item.get("version") == SBA_CAPABILITY_VERSION
                for item in closure
            )
        )
        combat_closure = [
            {"key": "rules/basic-priority", "version": "0.1.0"},
            {"key": "rules/combat-phase", "version": "0.1.0"},
            {"key": "rules/declare-attackers", "version": "0.1.0"},
            {"key": "rules/draw-card", "version": "0.1.0"},
            {"key": "rules/state-based-actions-combat", "version": "0.1.0"},
            {"key": "rules/turn-structure", "version": "0.1.0"},
            {"key": "rules/zone-incarnation", "version": "0.1.0"},
        ]
        combat_blockers_closure = [
            {"key": "rules/basic-priority", "version": "0.1.0"},
            {"key": "rules/combat-phase", "version": "0.1.0"},
            {"key": "rules/declare-attackers", "version": "0.1.0"},
            {"key": "rules/declare-blockers", "version": "0.1.0"},
            {"key": "rules/draw-card", "version": "0.1.0"},
            {"key": "rules/state-based-actions-combat", "version": "0.1.0"},
            {"key": "rules/turn-structure", "version": "0.1.0"},
            {"key": "rules/zone-incarnation", "version": "0.1.0"},
        ]
        exact_combat_profile = closure == combat_closure
        exact_combat_blockers_profile = closure == combat_blockers_closure
        expected_codec = (
            COMBAT_BLOCKERS_OBSERVATION_CODEC
            if exact_combat_blockers_profile
            else
            COMBAT_OBSERVATION_CODEC
            if exact_combat_profile
            else MAGIC_OBSERVATION_CODEC
            if magic_semantics_admitted
            else SYNTHETIC_OBSERVATION_CODEC
        )
        if self.schemas.observation_payload_codec != expected_codec:
            raise WireError(
                "semantic.replay_manifest",
                "observation payload codec does not match the exact rules contract closure",
            )
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
        if self.schemas.replay_step != REPLAY_STEP_SCHEMA_V6:
            raise WireError("semantic.replay_manifest", "replay-step schema is not V6")
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
        self.semantic_contract.validate()
        if (
            self.execution_identity.semantic_contract_id
            != self.semantic_contract.semantic_contract_id
        ):
            raise WireError(
                "semantic.replay_manifest", "execution identity semantic contract id does not match"
            )
        if self.execution_identity != self.initial_identity.execution_identity:
            raise WireError(
                "semantic.replay_manifest", "execution identity does not match initial identity"
            )
        if (
            self.initial_identity.execution_identity.semantic_contract_id
            != self.semantic_contract.semantic_contract_id
        ):
            raise WireError(
                "semantic.replay_manifest", "initial identity semantic contract id does not match"
            )
        rules_authority = self.semantic_contract.rules_manifest["rules_authority"]
        if not isinstance(rules_authority, dict):
            raise WireError("semantic.replay_manifest", "rules_authority must be a dict")
        if rules_authority["variant"] == "comprehensive_rules":
            if rules_authority.get("snapshot_id") != self.rules_snapshot:
                raise WireError(
                    "semantic.replay_manifest", "rules snapshot does not match manifest"
                )
        elif rules_authority["variant"] != "synthetic_legacy":
            raise WireError("semantic.replay_manifest", "unknown rules authority variant")
        self.initial_identity.validate(error_code="semantic.replay_manifest")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "card_bundle": require_nonempty(self.card_bundle, "card_bundle"),
            "decks": [deck.to_wire() for deck in self.decks],
            "engine_build": require_nonempty(self.engine_build, "engine_build"),
            "execution_identity": self.execution_identity.to_wire(),
            "format_policy_snapshot": require_nonempty(
                self.format_policy_snapshot, "format_policy_snapshot"
            ),
            "initial_identity": self.initial_identity.to_wire(),
            "kernel": self.kernel.to_wire(),
            "oracle_snapshot": require_nonempty(self.oracle_snapshot, "oracle_snapshot"),
            "randomness": self.randomness.to_wire(),
            "rules_snapshot": require_nonempty(self.rules_snapshot, "rules_snapshot"),
            "schema_version": REPLAY_MANIFEST_SCHEMA_V6,
            "schemas": self.schemas.to_wire(),
            "semantic_contract": self.semantic_contract.to_wire(),
        }


@dataclass(frozen=True, slots=True)
class ReplayStepV6:
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
    def from_wire(cls, value: object) -> ReplayStepV6:
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
class AuthoritativeReplayV6:
    schema_version: str
    manifest: ReplayManifestV6
    steps: tuple[ReplayStepV6, ...]
    final_identity: InitialEnvironmentIdentityV6

    @classmethod
    def from_wire(cls, value: object) -> AuthoritativeReplayV6:
        obj = require_exact_keys(value, {"schema_version", "manifest", "steps", "final_identity"})
        if not isinstance(obj["schema_version"], str) or not isinstance(obj["steps"], list):
            raise WireError("decode.invalid_json", "unsupported authoritative replay V6")
        try:
            manifest = ReplayManifestV6.from_wire(obj["manifest"])
        except WireError as exc:
            raise WireError("semantic.replay", exc.message) from exc
        result = cls(
            str(obj["schema_version"]),
            manifest,
            tuple(ReplayStepV6.from_wire(item) for item in obj["steps"]),
            InitialEnvironmentIdentityV6.from_wire(obj["final_identity"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if self.schema_version != REPLAY_FILE_SCHEMA_V6:
            raise WireError("semantic.replay", "unsupported authoritative replay V6")
        self.manifest.validate()
        manifest_initial = self.manifest.initial_identity
        if (
            self.manifest.execution_identity != manifest_initial.execution_identity
            or manifest_initial.execution_identity != self.final_identity.execution_identity
        ):
            raise WireError("semantic.replay", "execution identity is not consistent across replay")
        if (
            manifest_initial.execution_identity.semantic_contract_id
            != self.manifest.semantic_contract.semantic_contract_id
        ):
            raise WireError(
                "semantic.replay", "semantic contract id is not consistent across replay"
            )
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
                closure = self.manifest.semantic_contract.rules_manifest.get("capability_closure")
                closure_pairs = (
                    [(item.get("key"), item.get("version")) for item in closure]
                    if isinstance(closure, list) and all(isinstance(item, dict) for item in closure)
                    else []
                )
                priority_closure = [
                    ("rules/basic-priority", "0.1.0"),
                    ("rules/state-based-actions-combat", "0.1.0"),
                    ("rules/turn-structure", "0.1.0"),
                    ("rules/zone-incarnation", "0.1.0"),
                ]
                draw_closure = [
                    ("rules/basic-priority", "0.1.0"),
                    ("rules/draw-card", "0.1.0"),
                    ("rules/state-based-actions-combat", "0.1.0"),
                    ("rules/turn-structure", "0.1.0"),
                    ("rules/zone-incarnation", "0.1.0"),
                ]
                combat_closure = [
                    ("rules/basic-priority", "0.1.0"),
                    ("rules/combat-phase", "0.1.0"),
                    ("rules/declare-attackers", "0.1.0"),
                    ("rules/draw-card", "0.1.0"),
                    ("rules/state-based-actions-combat", "0.1.0"),
                    ("rules/turn-structure", "0.1.0"),
                    ("rules/zone-incarnation", "0.1.0"),
                ]
                maximum_advance = (
                    3
                    if closure_pairs == draw_closure
                    else 2
                    if closure_pairs in (priority_closure, combat_closure)
                    else 1
                )
                if (
                    not previous.state_revision
                    < step.state_revision_after
                    <= previous.state_revision + maximum_advance
                ):
                    raise WireError(
                        "semantic.replay", "accepted step revision exceeds its exact contract bound"
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
            previous = InitialEnvironmentIdentityV6(
                step.state_revision_after,
                step.full_state_digest_after,
                step.episode_status_after,
                step.environment_limit_counters_after,
                previous.checkpoint_codec_identity,
                step.checkpoint_digest_after,
                previous.execution_identity,
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
            "schema_version": REPLAY_FILE_SCHEMA_V6,
            "steps": [step.to_wire() for step in self.steps],
        }
