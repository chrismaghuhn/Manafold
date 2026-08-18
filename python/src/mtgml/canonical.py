from __future__ import annotations

import base64
import json
import re
from collections.abc import Mapping
from typing import Any

from .errors import WireError

_CANONICAL_UINT_RE = re.compile(r"0|[1-9][0-9]*\Z")
_DIGEST_RE = re.compile(r"[0-9a-f]{64}\Z")


def parse_uint(value: object, *, maximum: int = 2**64 - 1) -> int:
    if not isinstance(value, str) or _CANONICAL_UINT_RE.fullmatch(value) is None:
        raise WireError("decode.invalid_json", "expected canonical unsigned decimal string")
    parsed = int(value)
    if parsed > maximum:
        raise WireError("decode.invalid_json", "unsigned integer is out of range")
    return parsed


def uint_wire(value: int, *, maximum: int = 2**64 - 1) -> str:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0 or value > maximum:
        raise WireError("encode.serialization", "unsigned integer is out of range")
    return str(value)



def parse_u64_number(value: object) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0 or value > 2**64 - 1:
        raise WireError("decode.invalid_json", "expected unsigned 64-bit JSON integer")
    return value


def require_digest(value: object) -> str:
    if not isinstance(value, str) or _DIGEST_RE.fullmatch(value) is None:
        raise WireError("decode.invalid_json", "digest is not canonical lowercase SHA-256 hex")
    return value


def require_nonempty(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise WireError("decode.invalid_json", f"{label} must be a non-empty string")
    return value


def require_exact_keys(value: object, required: set[str], optional: set[str] | None = None) -> Mapping[str, Any]:
    if not isinstance(value, dict):
        raise WireError("decode.invalid_json", "expected object")
    optional = optional or set()
    keys = set(value)
    if not required.issubset(keys) or not keys.issubset(required | optional):
        raise WireError("decode.invalid_json", "object fields do not match the closed contract")
    return value


def require_canonical_base64(value: object) -> str:
    if not isinstance(value, str):
        raise WireError("decode.invalid_json", "base64 payload must be a string")
    try:
        decoded = base64.b64decode(value, validate=True)
    except (ValueError, base64.binascii.Error) as exc:
        raise WireError("semantic.observation", "payload is not canonical base64") from exc
    if base64.b64encode(decoded).decode("ascii") != value:
        raise WireError("semantic.observation", "payload is not canonical base64")
    return value


def canonical_json_bytes(value: object) -> bytes:
    try:
        return json.dumps(
            value,
            ensure_ascii=False,
            allow_nan=False,
            separators=(",", ":"),
            sort_keys=True,
        ).encode("utf-8")
    except (TypeError, ValueError) as exc:
        raise WireError("encode.serialization", str(exc)) from exc
