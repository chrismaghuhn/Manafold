from __future__ import annotations

import base64
import binascii
from dataclasses import dataclass

from ._replay_v5 import SemanticContractMaterialV5
from .canonical import (
    require_digest,
    require_exact_keys,
)
from .content_contract_v1 import decode_content_contract_manifest_v1
from .errors import WireError
from .persistence import (
    calculate_content_contract_id_v1,
    calculate_rules_contract_id_v1,
    calculate_semantic_contract_id_v1,
)

MAX_CONTENT_MANIFEST_BASE64_CHARS = 89_478_488


MAX_CONTENT_MANIFEST_BYTES = 64 * 1024 * 1024


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
        canonical = decode_content_contract_manifest_v1(raw)
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
