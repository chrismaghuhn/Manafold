from __future__ import annotations

import base64
import binascii
from dataclasses import dataclass

from ._replay_common import DeckIdentityV1, KernelIdentityV1
from ._replay_v2 import RandomnessIdentityV2
from ._replay_v4 import CheckpointCodecIdentityV4, EnvironmentLimitCountersV4
from ._replay_v5 import ExecutionIdentityV1, SemanticContractMaterialV5
from ._replay_v6 import _validate_status_for_players
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
    CHECKPOINT_CODEC_ID_V7,
    CHECKPOINT_CODEC_VERSION_V7,
    calculate_checkpoint_digest_v7,
    calculate_content_contract_id_v1,
    calculate_rules_contract_id_v1,
    calculate_semantic_contract_id_v1,
)
from .persistence import (
    decode_canonical as decode_cbor,
)
from .persistence import (
    encode_canonical as encode_cbor,
)

REPLAY_MANIFEST_SCHEMA_V7 = "replay-manifest.v7"
REPLAY_FILE_SCHEMA_V7 = "authoritative-replay.v7"
REPLAY_STEP_SCHEMA_V7 = "replay-step.v7"
MAX_CONTENT_MANIFEST_BASE64_CHARS = 89_478_488
MAX_CONTENT_MANIFEST_BYTES = 64 * 1024 * 1024
_BASIC_LAND_CLOSURE = [
    ("rules/basic-land-mana", "0.1.0"),
    ("rules/basic-priority", "0.1.0"),
    ("rules/land-play", "0.1.0"),
    ("rules/mana-pool", "0.1.0"),
    ("rules/turn-structure", "0.1.0"),
    ("rules/zone-incarnation", "0.1.0"),
]


def _validate_basic_land_manifest(value: object) -> bytes:
    if not (
        isinstance(value, list)
        and len(value) == 3
        and value[0] == "content-contract-manifest.v1"
        and value[1] == "mtgml.content-contract.v1"
        and isinstance(value[2], list)
    ):
        raise WireError("semantic.replay_manifest", "content manifest identity/shape is invalid")
    definitions: list[int] = []
    for row in value[2]:
        if not isinstance(row, list) or len(row) != 7 or row[0] != "card-definition-envelope.v1":
            raise WireError("semantic.replay_manifest", "content definition shape is invalid")
        if type(row[1]) is not int or not 0 <= row[1] <= 2**64 - 1:
            raise WireError("semantic.replay_manifest", "content definition id is invalid")
        definitions.append(row[1])
        binding = row[4]
        if not (
            isinstance(binding, list)
            and len(binding) == 2
            and binding[0] == "profiled"
            and isinstance(binding[1], list)
            and len(binding[1]) == 2
            and binding[1][0] == "basic-land@1.0.0"
            and isinstance(binding[1][1], list)
            and len(binding[1][1]) == 2
            and binding[1][1][0] == "basic-land-profile.v1"
            and binding[1][1][1] in {"mountain", "plains"}
        ):
            raise WireError("semantic.replay_manifest", "content profile is invalid")
        face = row[2]
        subtype = binding[1][1][1]
        if not (
            isinstance(face, list)
            and len(face) == 1
            and isinstance(face[0], list)
            and len(face[0]) == 2
            and type(face[0][0]) is int
            and face[0][0] == 0
            and isinstance(face[0][1], list)
            and len(face[0][1]) == 7
            and face[0][1][3] == [["Basic"], ["Land"], [subtype.title()]]
        ):
            raise WireError("semantic.replay_manifest", "content face/profile shape is invalid")
        ability_keys = row[3]
        if not (
            isinstance(ability_keys, list)
            and len(ability_keys) == 1
            and isinstance(ability_keys[0], list)
            and len(ability_keys[0]) == 2
            and all(type(value) is int and value == 0 for value in ability_keys[0])
            and row[5:] == [[], []]
        ):
            raise WireError("semantic.replay_manifest", "content definition identity is invalid")
    if definitions != sorted(definitions) or len(set(definitions)) != len(definitions):
        raise WireError("semantic.replay_manifest", "content definitions are not canonical")
    return encode_cbor(value)


@dataclass(frozen=True, slots=True)
class ContentContractMaterialV1:
    content_contract_id: str
    manifest_canonical_cbor_base64: str

    @classmethod
    def from_wire(cls, value: object) -> ContentContractMaterialV1:
        obj = require_exact_keys(value, {"content_contract_id", "manifest_canonical_cbor_base64"})
        content_id = require_digest(obj["content_contract_id"])
        encoded = obj["manifest_canonical_cbor_base64"]
        if not isinstance(encoded, str) or len(encoded) > MAX_CONTENT_MANIFEST_BASE64_CHARS:
            raise WireError("semantic.replay_manifest", "content Base64 exceeds its bound")
        try:
            raw = base64.b64decode(encoded.encode("ascii"), validate=True)
        except (UnicodeEncodeError, ValueError, binascii.Error) as exc:
            raise WireError("semantic.replay_manifest", "content Base64 is invalid") from exc
        if (
            len(raw) > MAX_CONTENT_MANIFEST_BYTES
            or base64.b64encode(raw).decode("ascii") != encoded
        ):
            raise WireError("semantic.replay_manifest", "content Base64 is noncanonical")
        try:
            manifest = decode_cbor(raw)
        except ValueError as exc:
            raise WireError("semantic.replay_manifest", "content CBOR is invalid") from exc
        canonical = _validate_basic_land_manifest(manifest)
        if canonical != raw:
            raise WireError("semantic.replay_manifest", "content CBOR is noncanonical")
        if calculate_content_contract_id_v1(canonical) != content_id:
            raise WireError("semantic.replay_manifest", "content child digest does not match")
        return cls(content_id, encoded)

    def to_wire(self) -> dict[str, object]:
        validated = ContentContractMaterialV1.from_wire(
            {
                "content_contract_id": self.content_contract_id,
                "manifest_canonical_cbor_base64": self.manifest_canonical_cbor_base64,
            }
        )
        return {
            "content_contract_id": validated.content_contract_id,
            "manifest_canonical_cbor_base64": validated.manifest_canonical_cbor_base64,
        }


@dataclass(frozen=True, slots=True)
class SemanticContractMaterialV7:
    semantic_contract_id: str
    manifest: dict[str, object]
    rules_manifest: dict[str, object]
    content_contract: ContentContractMaterialV1 | None

    @classmethod
    def from_wire(cls, value: object) -> SemanticContractMaterialV7:
        obj = require_exact_keys(
            value, {"semantic_contract_id", "manifest", "rules_manifest", "content_contract"}
        )
        # Reuse the frozen V5 identity-field parser; V7 adds only the verified child.
        parent = SemanticContractMaterialV5.from_wire(
            {
                "semantic_contract_id": obj["semantic_contract_id"],
                "manifest": obj["manifest"],
                "rules_manifest": obj["rules_manifest"],
            }
        )
        child = (
            None
            if obj["content_contract"] is None
            else ContentContractMaterialV1.from_wire(obj["content_contract"])
        )
        result = cls(parent.semantic_contract_id, parent.manifest, parent.rules_manifest, child)
        result.validate()
        return result

    def validate(self) -> None:
        if self.manifest.get("format_contract_id") is not None:
            raise WireError("semantic.replay_manifest", "format contract must be absent in V7")
        try:
            if (
                calculate_rules_contract_id_v1(self.rules_manifest)
                != self.manifest["rules_contract_id"]
            ):
                raise WireError("semantic.replay_manifest", "rules contract ID does not match")
            if calculate_semantic_contract_id_v1(self.manifest) != self.semantic_contract_id:
                raise WireError("semantic.replay_manifest", "semantic contract ID does not match")
        except (KeyError, ValueError) as exc:
            raise WireError(
                "semantic.replay_manifest", "semantic contract material is invalid"
            ) from exc
        parent_content = self.manifest.get("content_contract_id")
        child_id = (
            None if self.content_contract is None else self.content_contract.content_contract_id
        )
        if parent_content != child_id:
            raise WireError(
                "semantic.replay_manifest", "content child presence or identity differs"
            )
        if self.content_contract is not None:
            self.content_contract.to_wire()

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "content_contract": None
            if self.content_contract is None
            else self.content_contract.to_wire(),
            "manifest": self.manifest,
            "rules_manifest": self.rules_manifest,
            "semantic_contract_id": require_digest(self.semantic_contract_id),
        }


@dataclass(frozen=True, slots=True)
class ReplaySchemaVersionsV7:
    observation: str
    observation_payload_codec: str
    information_state: str
    decision: str
    decision_response: str
    observed_event: str
    player_step: str
    replay_step: str

    @classmethod
    def from_wire(cls, value: object) -> ReplaySchemaVersionsV7:
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
class InitialEnvironmentIdentityV7:
    state_revision: int
    full_state_digest: str
    episode_status: EpisodeStatus
    environment_limit_counters: EnvironmentLimitCountersV4
    checkpoint_codec_identity: CheckpointCodecIdentityV4
    checkpoint_digest: str
    execution_identity: ExecutionIdentityV1

    @classmethod
    def from_wire(cls, value: object) -> InitialEnvironmentIdentityV7:
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
        return calculate_checkpoint_digest_v7(
            self.full_state_digest,
            self.episode_status,
            self.environment_limit_counters.as_dict(),
            self.checkpoint_codec_identity.codec_id,
            self.checkpoint_codec_identity.semantic_version,
            self.execution_identity.program_kind,
            self.execution_identity.semantic_contract_id,
        )

    def validate(self) -> None:
        if (
            self.checkpoint_codec_identity.codec_id != CHECKPOINT_CODEC_ID_V7
            or self.checkpoint_codec_identity.semantic_version != CHECKPOINT_CODEC_VERSION_V7
        ):
            raise WireError("semantic.replay_manifest", "checkpoint codec identity is not V7")
        self.episode_status.to_wire()
        counters = self.environment_limit_counters
        if counters.accepted_transitions > counters.decisions_submitted:
            raise WireError(
                "semantic.replay_manifest", "accepted transitions exceed submitted decisions"
            )
        if self.recompute_checkpoint_digest() != self.checkpoint_digest:
            raise WireError("semantic.replay_manifest", "checkpoint digest does not match")

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


def _program_authority_matches(program: str, authority: object) -> bool:
    if not isinstance(authority, dict):
        return False
    variant = authority.get("variant")
    return (program == "synthetic_rules_compat" and variant == "synthetic_legacy") or (
        program == "magic_rules" and variant == "comprehensive_rules"
    )


def _expected_payload_codec(program: str, semantic: SemanticContractMaterialV7) -> str:
    manifest = semantic.rules_manifest
    authority = manifest.get("rules_authority")
    closure = manifest.get("capability_closure")
    if not _program_authority_matches(program, authority):
        raise WireError(
            "semantic.replay_manifest", "execution program and rules authority do not match"
        )
    pairs = (
        [(item.get("key"), item.get("version")) for item in closure]
        if isinstance(closure, list) and all(isinstance(item, dict) for item in closure)
        else []
    )
    if pairs == _BASIC_LAND_CLOSURE and semantic.content_contract is not None:
        return "magic-basic-land-observation.v1"
    if semantic.content_contract is not None:
        raise WireError(
            "semantic.replay_manifest", "content child is outside the admitted basic-land closure"
        )
    if authority.get("variant") == "synthetic_legacy":
        if closure is not None:
            raise WireError("semantic.replay_manifest", "synthetic rules closure must be null")
        return "synthetic-m3-observation.v1"
    closures = {
        (
            "rules/basic-priority",
            "rules/state-based-actions-combat",
            "rules/turn-structure",
            "rules/zone-incarnation",
        ): "magic-m3-observation.v1",
        (
            "rules/basic-priority",
            "rules/draw-card",
            "rules/state-based-actions-combat",
            "rules/turn-structure",
            "rules/zone-incarnation",
        ): "magic-m3-observation.v1",
        (
            "rules/basic-priority",
            "rules/combat-phase",
            "rules/declare-attackers",
            "rules/draw-card",
            "rules/state-based-actions-combat",
            "rules/turn-structure",
            "rules/zone-incarnation",
        ): "magic-combat-observation.v2",
        (
            "rules/basic-priority",
            "rules/combat-phase",
            "rules/declare-attackers",
            "rules/declare-blockers",
            "rules/draw-card",
            "rules/state-based-actions-combat",
            "rules/turn-structure",
            "rules/zone-incarnation",
        ): "magic-combat-observation.v3",
        (
            "rules/basic-priority",
            "rules/combat-damage",
            "rules/combat-phase",
            "rules/damage-and-life",
            "rules/declare-attackers",
            "rules/declare-blockers",
            "rules/draw-card",
            "rules/state-based-actions-combat",
            "rules/turn-structure",
            "rules/zone-incarnation",
        ): "magic-combat-observation.v4",
    }
    keys = tuple(key for key, _ in pairs)
    if all(version == "0.1.0" for _, version in pairs) and keys in closures:
        return closures[keys]
    if any(
        key == "rules/state-based-actions-combat" and version == "0.1.0" for key, version in pairs
    ):
        excluded = {
            "rules/combat-phase",
            "rules/declare-attackers",
            "rules/declare-blockers",
            "rules/combat-damage",
            "rules/cleanup-reset",
        }
        if not any(key in excluded for key, _ in pairs):
            return "magic-m3-observation.v1"
    raise WireError(
        "semantic.replay_manifest", "observation codec is not admitted for this rules contract"
    )


def _forced_progress_budget(manifest: ReplayManifestV7) -> int:
    authority = manifest.semantic_contract.rules_manifest["rules_authority"]
    closure = manifest.semantic_contract.rules_manifest["capability_closure"]
    if (
        manifest.execution_identity.program_kind != "magic_rules"
        or not isinstance(authority, dict)
        or authority.get("variant") != "comprehensive_rules"
        or not isinstance(closure, list)
    ):
        return 0
    pairs = [(entry["key"], entry["version"]) for entry in closure]
    one = [
        ("rules/basic-priority", "0.1.0"),
        ("rules/state-based-actions-combat", "0.1.0"),
        ("rules/turn-structure", "0.1.0"),
        ("rules/zone-incarnation", "0.1.0"),
    ]
    two = [
        ("rules/basic-priority", "0.1.0"),
        ("rules/draw-card", "0.1.0"),
        ("rules/state-based-actions-combat", "0.1.0"),
        ("rules/turn-structure", "0.1.0"),
        ("rules/zone-incarnation", "0.1.0"),
    ]
    combat = [
        ("rules/basic-priority", "0.1.0"),
        ("rules/combat-phase", "0.1.0"),
        ("rules/declare-attackers", "0.1.0"),
        ("rules/draw-card", "0.1.0"),
        ("rules/state-based-actions-combat", "0.1.0"),
        ("rules/turn-structure", "0.1.0"),
        ("rules/zone-incarnation", "0.1.0"),
    ]
    blockers = [
        ("rules/basic-priority", "0.1.0"),
        ("rules/combat-phase", "0.1.0"),
        ("rules/declare-attackers", "0.1.0"),
        ("rules/declare-blockers", "0.1.0"),
        ("rules/draw-card", "0.1.0"),
        ("rules/state-based-actions-combat", "0.1.0"),
        ("rules/turn-structure", "0.1.0"),
        ("rules/zone-incarnation", "0.1.0"),
    ]
    damage = [
        ("rules/basic-priority", "0.1.0"),
        ("rules/combat-damage", "0.1.0"),
        ("rules/combat-phase", "0.1.0"),
        ("rules/damage-and-life", "0.1.0"),
        ("rules/declare-attackers", "0.1.0"),
        ("rules/declare-blockers", "0.1.0"),
        ("rules/draw-card", "0.1.0"),
        ("rules/state-based-actions-combat", "0.1.0"),
        ("rules/turn-structure", "0.1.0"),
        ("rules/zone-incarnation", "0.1.0"),
    ]
    bounded = [
        ("rules/basic-priority", "0.1.0"),
        ("rules/cleanup-reset", "0.1.0"),
        ("rules/combat-damage", "0.1.0"),
        ("rules/combat-phase", "0.1.0"),
        ("rules/damage-and-life", "0.1.0"),
        ("rules/declare-attackers", "0.1.0"),
        ("rules/declare-blockers", "0.1.0"),
        ("rules/draw-card", "0.1.0"),
        ("rules/state-based-actions-combat", "0.1.0"),
        ("rules/turn-structure", "0.1.0"),
        ("rules/zone-incarnation", "0.1.0"),
    ]
    if pairs in (combat, blockers, one):
        return 1
    if pairs == damage:
        return 2
    if pairs == two:
        return 2
    if pairs == bounded:
        return 1
    return 0


@dataclass(frozen=True, slots=True)
class ReplayManifestV7:
    schema_version: str
    engine_build: str
    kernel: KernelIdentityV1
    rules_snapshot: str
    format_policy_snapshot: str
    oracle_snapshot: str
    card_bundle: str
    schemas: ReplaySchemaVersionsV7
    randomness: RandomnessIdentityV2
    decks: tuple[DeckIdentityV1, ...]
    initial_identity: InitialEnvironmentIdentityV7
    execution_identity: ExecutionIdentityV1
    semantic_contract: SemanticContractMaterialV7

    @classmethod
    def from_wire(cls, value: object) -> ReplayManifestV7:
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
        if not isinstance(obj["decks"], list):
            raise WireError("decode.invalid_json", "decks must be an array")
        result = cls(
            obj["schema_version"],
            require_nonempty(obj["engine_build"], "engine_build"),
            KernelIdentityV1.from_wire(obj["kernel"]),
            require_nonempty(obj["rules_snapshot"], "rules_snapshot"),
            require_nonempty(obj["format_policy_snapshot"], "format_policy_snapshot"),
            require_nonempty(obj["oracle_snapshot"], "oracle_snapshot"),
            require_nonempty(obj["card_bundle"], "card_bundle"),
            ReplaySchemaVersionsV7.from_wire(obj["schemas"]),
            RandomnessIdentityV2.from_wire(obj["randomness"]),
            tuple(DeckIdentityV1.from_wire(d) for d in obj["decks"]),
            InitialEnvironmentIdentityV7.from_wire(obj["initial_identity"]),
            ExecutionIdentityV1.from_wire(obj["execution_identity"]),
            SemanticContractMaterialV7.from_wire(obj["semantic_contract"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if self.schema_version != REPLAY_MANIFEST_SCHEMA_V7:
            raise WireError("semantic.replay_manifest", "unsupported replay manifest V7")
        if self.randomness.contract_id != "mtgml.rng.v1":
            raise WireError("semantic.replay_manifest", "unsupported RNG contract")
        schema_expected = {
            "observation": "observation-envelope.v1",
            "information_state": "information-state-envelope.v2",
            "decision": "player-decision-request.v3",
            "decision_response": "decision-response.v2",
            "observed_event": "observed-event-envelope.v3",
            "player_step": "player-step.v3",
            "replay_step": REPLAY_STEP_SCHEMA_V7,
        }
        for key, expected in schema_expected.items():
            if getattr(self.schemas, key) != expected:
                raise WireError(
                    "semantic.replay_manifest", f"schema identity {key} is not V7 profile"
                )
        if self.schemas.observation_payload_codec != _expected_payload_codec(
            self.execution_identity.program_kind, self.semantic_contract
        ):
            raise WireError(
                "semantic.replay_manifest",
                "observation payload codec does not match rules identity",
            )
        if not self.decks:
            raise WireError("semantic.replay_manifest", "decks must not be empty")
        players = [d.player for d in self.decks]
        if (
            any(not d.deck_id for d in self.decks)
            or len(players) != len(set(players))
            or players != sorted(players)
        ):
            raise WireError("semantic.replay_manifest", "deck identities are not canonical")
        self.semantic_contract.validate()
        if (
            self.execution_identity.semantic_contract_id
            != self.semantic_contract.semantic_contract_id
            or self.initial_identity.execution_identity != self.execution_identity
        ):
            raise WireError(
                "semantic.replay_manifest", "execution identity does not match semantic contract"
            )
        authority = self.semantic_contract.rules_manifest["rules_authority"]
        if (
            authority["variant"] == "comprehensive_rules"
            and authority.get("snapshot_id") != self.rules_snapshot
        ):
            raise WireError("semantic.replay_manifest", "rules snapshot does not match")
        _validate_status_for_players(self.initial_identity.episode_status, set(players))
        self.initial_identity.validate()

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "card_bundle": self.card_bundle,
            "decks": [d.to_wire() for d in self.decks],
            "engine_build": self.engine_build,
            "execution_identity": self.execution_identity.to_wire(),
            "format_policy_snapshot": self.format_policy_snapshot,
            "initial_identity": self.initial_identity.to_wire(),
            "kernel": self.kernel.to_wire(),
            "oracle_snapshot": self.oracle_snapshot,
            "randomness": self.randomness.to_wire(),
            "rules_snapshot": self.rules_snapshot,
            "schema_version": REPLAY_MANIFEST_SCHEMA_V7,
            "schemas": self.schemas.to_wire(),
            "semantic_contract": self.semantic_contract.to_wire(),
        }


@dataclass(frozen=True, slots=True)
class ReplayStepV7:
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
    def from_wire(cls, value: object) -> ReplayStepV7:
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
class AuthoritativeReplayV7:
    schema_version: str
    manifest: ReplayManifestV7
    steps: tuple[ReplayStepV7, ...]
    final_identity: InitialEnvironmentIdentityV7

    @classmethod
    def from_wire(cls, value: object) -> AuthoritativeReplayV7:
        obj = require_exact_keys(value, {"schema_version", "manifest", "steps", "final_identity"})
        if not isinstance(obj["steps"], list):
            raise WireError("decode.invalid_json", "steps must be an array")
        result = cls(
            obj["schema_version"],
            ReplayManifestV7.from_wire(obj["manifest"]),
            tuple(ReplayStepV7.from_wire(s) for s in obj["steps"]),
            InitialEnvironmentIdentityV7.from_wire(obj["final_identity"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if self.schema_version != REPLAY_FILE_SCHEMA_V7:
            raise WireError("semantic.replay", "unsupported authoritative replay V7")
        self.manifest.validate()
        previous = self.manifest.initial_identity
        players = {d.player for d in self.manifest.decks}
        for index, step in enumerate(self.steps):
            if (
                step.step_index != index
                or step.actor not in players
                or step.checkpoint_digest_before != previous.checkpoint_digest
                or step.state_revision_before != previous.state_revision
                or step.response.state_revision != previous.state_revision
            ):
                raise WireError("semantic.replay", "replay step linkage is discontinuous")
            step.response.validate()
            before = previous.environment_limit_counters
            after = step.environment_limit_counters_after
            if not step.accepted:
                if (
                    step.state_revision_after != previous.state_revision
                    or step.full_state_digest_after != previous.full_state_digest
                    or step.episode_status_after != previous.episode_status
                    or after != before
                    or step.checkpoint_digest_after != previous.checkpoint_digest
                ):
                    raise WireError("semantic.replay", "rejected step mutated identity")
            else:
                if (
                    not previous.state_revision
                    < step.state_revision_after
                    <= previous.state_revision + 1 + _forced_progress_budget(self.manifest)
                ):
                    raise WireError("semantic.replay", "accepted step revision is invalid")
                if (
                    after.decisions_submitted != before.decisions_submitted + 1
                    or after.accepted_transitions != before.accepted_transitions + 1
                    or after.accepted_transitions > after.decisions_submitted
                    or after.rule_events_emitted < before.rule_events_emitted
                    or after.resource_units_consumed < before.resource_units_consumed
                    or after.wall_clock_elapsed_millis < before.wall_clock_elapsed_millis
                ):
                    raise WireError("semantic.replay", "accepted step counters are discontinuous")
            previous = InitialEnvironmentIdentityV7(
                step.state_revision_after,
                step.full_state_digest_after,
                step.episode_status_after,
                after,
                previous.checkpoint_codec_identity,
                step.checkpoint_digest_after,
                previous.execution_identity,
            )
            previous.validate()
            _validate_status_for_players(previous.episode_status, players)
        if self.final_identity != previous:
            raise WireError("semantic.replay", "final identity does not match last step")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "final_identity": self.final_identity.to_wire(),
            "manifest": self.manifest.to_wire(),
            "schema_version": REPLAY_FILE_SCHEMA_V7,
            "steps": [s.to_wire() for s in self.steps],
        }


@dataclass(slots=True)
class ReplayRecorderV7:
    manifest: ReplayManifestV7
    steps: list[ReplayStepV7]
    final_identity: InitialEnvironmentIdentityV7

    @classmethod
    def create(cls, manifest: ReplayManifestV7) -> ReplayRecorderV7:
        manifest.validate()
        return cls(manifest, [], manifest.initial_identity)

    def append(self, step: ReplayStepV7) -> None:
        candidate_steps = (*self.steps, step)
        final = InitialEnvironmentIdentityV7(
            step.state_revision_after,
            step.full_state_digest_after,
            step.episode_status_after,
            step.environment_limit_counters_after,
            self.final_identity.checkpoint_codec_identity,
            step.checkpoint_digest_after,
            self.final_identity.execution_identity,
        )
        AuthoritativeReplayV7(
            REPLAY_FILE_SCHEMA_V7, self.manifest, candidate_steps, final
        ).validate()
        self.steps.append(step)
        self.final_identity = final

    def export(self) -> AuthoritativeReplayV7:
        replay = AuthoritativeReplayV7(
            REPLAY_FILE_SCHEMA_V7, self.manifest, tuple(self.steps), self.final_identity
        )
        replay.validate()
        return replay
