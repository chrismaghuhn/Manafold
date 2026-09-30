from __future__ import annotations

import base64
import hashlib
from dataclasses import dataclass

from .canonical import (
    parse_uint,
    require_canonical_base64,
    require_digest,
    require_exact_keys,
    require_nonempty,
    uint_wire,
)
from .errors import WireError

OBSERVATION_SCHEMA = "observation-envelope.v1"


def observation_digest_from_payload(payload: bytes) -> str:
    return hashlib.sha256(b"mtgml.observation-digest.v1\x00" + payload).hexdigest()


@dataclass(frozen=True, slots=True)
class ObservationEnvelope:
    schema_version: str
    perspective: int
    state_revision: int
    payload_codec: str
    payload_base64: str
    digest: str

    @classmethod
    def from_wire(cls, value: object) -> ObservationEnvelope:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "perspective",
                "state_revision",
                "payload_codec",
                "payload_base64",
                "digest",
            },
        )
        if obj["schema_version"] != OBSERVATION_SCHEMA:
            raise WireError("decode.invalid_json", "unsupported observation schema")
        result = cls(
            OBSERVATION_SCHEMA,
            parse_uint(obj["perspective"]),
            parse_uint(obj["state_revision"]),
            require_nonempty(obj["payload_codec"], "payload_codec"),
            require_canonical_base64(obj["payload_base64"]),
            require_digest(obj["digest"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        payload_base64 = require_canonical_base64(self.payload_base64)
        digest = require_digest(self.digest)
        payload = base64.b64decode(payload_base64, validate=True)
        if observation_digest_from_payload(payload) != digest:
            raise WireError(
                "semantic.observation",
                "observation digest does not match payload",
            )

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "digest": require_digest(self.digest),
            "payload_base64": require_canonical_base64(self.payload_base64),
            "payload_codec": require_nonempty(self.payload_codec, "payload_codec"),
            "perspective": uint_wire(self.perspective),
            "schema_version": self.schema_version,
            "state_revision": uint_wire(self.state_revision),
        }
