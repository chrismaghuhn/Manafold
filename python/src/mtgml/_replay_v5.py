from __future__ import annotations

from dataclasses import dataclass

from .canonical import (
    require_digest,
    require_exact_keys,
    require_nonempty,
)
from .errors import WireError
from .persistence import (
    PersistenceError,
    calculate_rules_contract_id_v1,
    calculate_semantic_contract_id_v1,
)

_VALID_PROGRAM_KINDS = frozenset({"magic_rules"})


@dataclass(frozen=True, slots=True)
class ExecutionIdentityV1:
    program_kind: str
    semantic_contract_id: str

    @classmethod
    def from_wire(cls, value: object) -> ExecutionIdentityV1:
        obj = require_exact_keys(value, {"program_kind", "semantic_contract_id"})
        result = cls(
            program_kind=require_nonempty(obj["program_kind"], "program_kind"),
            semantic_contract_id=require_digest(obj["semantic_contract_id"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        require_nonempty(self.program_kind, "program_kind")
        if self.program_kind not in _VALID_PROGRAM_KINDS:
            raise WireError("decode.invalid_json", "unknown execution program kind")
        require_digest(self.semantic_contract_id)

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "program_kind": self.program_kind,
            "semantic_contract_id": self.semantic_contract_id,
        }


def _require_contract_manifest(value: object) -> dict[str, object]:
    if not isinstance(value, dict):
        raise WireError("decode.invalid_json", "semantic contract manifest must be an object")
    return dict(value)


def _require_rules_manifest(value: object) -> dict[str, object]:
    if not isinstance(value, dict):
        raise WireError("decode.invalid_json", "rules contract manifest must be an object")
    return dict(value)


@dataclass(frozen=True, slots=True)
class SemanticContractMaterialV5:
    semantic_contract_id: str
    manifest: dict[str, object]
    rules_manifest: dict[str, object]

    @classmethod
    def from_wire(cls, value: object) -> SemanticContractMaterialV5:
        obj = require_exact_keys(
            value,
            {"semantic_contract_id", "manifest", "rules_manifest"},
        )
        manifest = _require_contract_manifest(obj["manifest"])
        rules_manifest = _require_rules_manifest(obj["rules_manifest"])
        return cls(
            semantic_contract_id=require_digest(obj["semantic_contract_id"]),
            manifest=manifest,
            rules_manifest=rules_manifest,
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "semantic_contract_id": require_digest(self.semantic_contract_id),
            "manifest": self.manifest,
            "rules_manifest": self.rules_manifest,
        }

    def validate(self) -> None:
        try:
            recomputed_semantic = calculate_semantic_contract_id_v1(self.manifest)
        except PersistenceError as exc:
            raise WireError(
                "semantic.replay_manifest", "semantic contract manifest is invalid"
            ) from exc
        if recomputed_semantic != self.semantic_contract_id:
            raise WireError("semantic.replay_manifest", "semantic contract id does not match")
        try:
            recomputed_rules = calculate_rules_contract_id_v1(self.rules_manifest)
        except PersistenceError as exc:
            raise WireError(
                "semantic.replay_manifest", "rules contract manifest is invalid"
            ) from exc
        if recomputed_rules != self.manifest["rules_contract_id"]:
            raise WireError("semantic.replay_manifest", "rules contract id does not match")
        manifest = self.manifest
        if (
            manifest["format_contract_id"] is not None
            or manifest["content_contract_id"] is not None
        ):
            raise WireError(
                "semantic.replay_manifest", "format/content contract ids must be null in V5"
            )
