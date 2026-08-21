"""Mechanical parity for Manafold's restricted persisted semantic codec.

This module deliberately exposes bytes, envelopes, and detached value shapes
only.  It does not decode EngineState or perform rules/state validation.
"""

from __future__ import annotations

import hashlib
import struct
from typing import TypeAlias

from .canonical import require_digest
from .episode import EpisodeStatus

MAX_PAYLOAD_BYTES = 64 * 1024 * 1024
MAX_TEXT_BYTES = 1024 * 1024
MAX_BYTE_STRING_BYTES = 64 * 1024 * 1024
MAX_ARRAY_ELEMENTS = 1024 * 1024
MAX_DEPTH = 64
MAX_ITEMS = 4 * 1024 * 1024

DIGEST_ENVELOPE_ID = "mtgml.digest-envelope.v1"
SHA256_ID = "sha-256"
CANONICAL_CBOR_ID = "mtgml.canonical-cbor.v1"
MAX_IDENTIFIER_BYTES = 255
CHECKPOINT_DOMAIN = "mtgml.checkpoint-digest.v3"
CHECKPOINT_INPUT_SCHEMA = "environment-checkpoint-digest-input.v3"

PersistenceValue: TypeAlias = None | bool | int | bytes | str | list["PersistenceValue"]


class PersistenceError(ValueError):
    def __init__(self, code: str, message: str = "") -> None:
        super().__init__(f"{code}: {message}" if message else code)
        self.code = code
        self.message = message


def _error(code: str, message: str) -> PersistenceError:
    return PersistenceError(code, message)


def _write_head(major: int, value: int) -> bytes:
    if value < 0 or value > 2**64 - 1:
        raise _error("value_out_of_range", "CBOR argument is outside u64")
    prefix = major << 5
    if value <= 23:
        return bytes([prefix | value])
    if value <= 0xFF:
        return bytes([prefix | 24, value])
    if value <= 0xFFFF:
        return bytes([prefix | 25]) + value.to_bytes(2, "big")
    if value <= 0xFFFFFFFF:
        return bytes([prefix | 26]) + value.to_bytes(4, "big")
    return bytes([prefix | 27]) + value.to_bytes(8, "big")


def _encode_value(value: PersistenceValue, depth: int, items: list[int]) -> bytes:
    items[0] += 1
    if items[0] > MAX_ITEMS:
        raise _error("item_limit_exceeded", "CBOR item limit exceeded")
    if value is None:
        return b"\xf6"
    if value is False:
        return b"\xf4"
    if value is True:
        return b"\xf5"
    if isinstance(value, int):
        if isinstance(value, bool):
            raise _error("disallowed_cbor_form", "boolean is not an integer")
        if 0 <= value <= 2**64 - 1:
            return _write_head(0, value)
        if -(2**63) <= value < 0:
            return _write_head(1, -1 - value)
        raise _error("value_out_of_range", "signed integer is outside i64")
    if isinstance(value, bytes):
        if len(value) > MAX_BYTE_STRING_BYTES:
            raise _error("payload_too_large", "byte string is too large")
        return _write_head(2, len(value)) + value
    if isinstance(value, str):
        encoded = value.encode("utf-8")
        if len(encoded) > MAX_TEXT_BYTES:
            raise _error("string_too_large", "text string is too large")
        return _write_head(3, len(encoded)) + encoded
    if isinstance(value, list):
        if len(value) > MAX_ARRAY_ELEMENTS:
            raise _error("array_too_large", "array is too large")
        if depth >= MAX_DEPTH:
            raise _error("depth_exceeded", "CBOR nesting is too deep")
        return _write_head(4, len(value)) + b"".join(
            _encode_value(item, depth + 1, items) for item in value
        )
    raise _error("disallowed_cbor_form", "unsupported persistence value")


def encode_canonical(value: PersistenceValue) -> bytes:
    encoded = _encode_value(value, 0, [0])
    if len(encoded) > MAX_PAYLOAD_BYTES:
        raise _error("payload_too_large", "payload is too large")
    return encoded


class _Decoder:
    def __init__(self, data: bytes) -> None:
        self.data = data
        self.offset = 0
        self.items = 0

    def exact(self, length: int) -> bytes:
        end = self.offset + length
        if length < 0 or end > len(self.data):
            raise _error("envelope_length", "CBOR value is truncated")
        result = self.data[self.offset : end]
        self.offset = end
        return result

    def head(self) -> tuple[int, int]:
        first = self.exact(1)[0]
        major, additional = first >> 5, first & 0x1F
        if additional <= 23:
            return major, additional
        if additional == 31:
            raise _error("disallowed_cbor_form", "indefinite values are forbidden")
        widths = {24: 1, 25: 2, 26: 4, 27: 8}
        width = widths.get(additional)
        if width is None:
            raise _error("disallowed_cbor_form", "CBOR additional information is forbidden")
        raw = int.from_bytes(self.exact(width), "big")
        minimum = {1: 24, 2: 256, 4: 65536, 8: 2**32}[width]
        if raw < minimum:
            raise _error("noncanonical_primitive", "CBOR integer is not shortest")
        return major, raw

    def value(self, depth: int) -> PersistenceValue:
        self.items += 1
        if self.items > MAX_ITEMS:
            raise _error("item_limit_exceeded", "CBOR item limit exceeded")
        major, argument = self.head()
        if major == 0:
            return argument
        if major == 1:
            if argument > 2**63 - 1:
                raise _error("value_out_of_range", "negative integer is outside i64")
            return -1 - argument
        if major == 2:
            if argument > MAX_BYTE_STRING_BYTES:
                raise _error("payload_too_large", "byte string is too large")
            return self.exact(argument)
        if major == 3:
            if argument > MAX_TEXT_BYTES:
                raise _error("string_too_large", "text string is too large")
            try:
                return self.exact(argument).decode("utf-8")
            except UnicodeDecodeError as exc:
                raise _error("invalid_utf8", "text is not valid UTF-8") from exc
        if major == 4:
            if argument > MAX_ARRAY_ELEMENTS:
                raise _error("array_too_large", "array is too large")
            if depth >= MAX_DEPTH:
                raise _error("depth_exceeded", "CBOR nesting is too deep")
            if self.items + argument > MAX_ITEMS:
                raise _error("item_limit_exceeded", "CBOR item limit exceeded")
            return [self.value(depth + 1) for _ in range(argument)]
        if major == 7 and argument in {20, 21, 22}:
            return {20: False, 21: True, 22: None}[argument]
        raise _error("disallowed_cbor_form", "CBOR maps, tags, and floats are forbidden")


def decode_canonical(payload: bytes) -> PersistenceValue:
    if len(payload) > MAX_PAYLOAD_BYTES:
        raise _error("payload_too_large", "payload is too large")
    decoder = _Decoder(payload)
    value = decoder.value(0)
    if decoder.offset != len(payload):
        raise _error("trailing_data", "trailing bytes follow value")
    if encode_canonical(value) != payload:
        raise _error("reencode_mismatch", "value does not re-encode canonically")
    return value


def _identifier(value: str) -> bytes:
    if not isinstance(value, str):
        raise _error("envelope_identity", "identifier is not text")
    encoded = value.encode("utf-8")
    if not encoded or len(encoded) > MAX_IDENTIFIER_BYTES or any(byte > 0x7F for byte in encoded):
        raise _error("envelope_identity", "identifier is not non-empty ASCII")
    return encoded


def _frame(value: bytes) -> bytes:
    return struct.pack(">Q", len(value)) + value


def encode_envelope(semantic_domain: str, input_schema_id: str, payload: bytes) -> bytes:
    domain = _identifier(semantic_domain)
    schema = _identifier(input_schema_id)
    if len(payload) > MAX_PAYLOAD_BYTES:
        raise _error("payload_too_large", "payload is too large")
    decode_canonical(payload)
    return (
        DIGEST_ENVELOPE_ID.encode("ascii")
        + b"\0"
        + _frame(SHA256_ID.encode("ascii"))
        + _frame(domain)
        + _frame(CANONICAL_CBOR_ID.encode("ascii"))
        + _frame(schema)
        + _frame(payload)
    )


def hash_envelope(envelope: bytes) -> bytes:
    return hashlib.sha256(envelope).digest()


def decode_envelope(envelope: bytes) -> tuple[dict[str, object], bytes]:
    prefix = DIGEST_ENVELOPE_ID.encode("ascii") + b"\0"
    if not envelope.startswith(prefix):
        raise _error("envelope_identity", "envelope prefix is invalid")
    offset = len(prefix)

    def read_frame(identifier: bool) -> bytes:
        nonlocal offset
        if offset + 8 > len(envelope):
            raise _error("envelope_length", "envelope frame length is truncated")
        length = struct.unpack(">Q", envelope[offset : offset + 8])[0]
        offset += 8
        limit = MAX_IDENTIFIER_BYTES if identifier else MAX_PAYLOAD_BYTES
        if length > limit:
            raise _error(
                "envelope_identity" if identifier else "payload_too_large", "frame is too large"
            )
        end = offset + length
        if end > len(envelope):
            raise _error("envelope_length", "envelope frame is truncated")
        value = envelope[offset:end]
        offset = end
        return value

    algorithm = read_frame(True)
    domain = read_frame(True)
    codec = read_frame(True)
    schema = read_frame(True)
    payload = read_frame(False)
    if offset != len(envelope):
        raise _error("envelope_length", "trailing envelope bytes")
    try:
        algorithm_text = algorithm.decode("ascii")
        domain_text = domain.decode("ascii")
        codec_text = codec.decode("ascii")
        schema_text = schema.decode("ascii")
    except UnicodeDecodeError as exc:
        raise _error("envelope_identity", "identifier is not ASCII") from exc
    if algorithm_text != SHA256_ID or codec_text != CANONICAL_CBOR_ID:
        raise _error("envelope_identity", "unsupported envelope algorithm or codec")
    _identifier(domain_text)
    _identifier(schema_text)
    decode_canonical(payload)
    reference: dict[str, object] = {
        "envelope_version": DIGEST_ENVELOPE_ID,
        "algorithm_id": algorithm_text,
        "semantic_domain": domain_text,
        "payload_codec_id": codec_text,
        "input_schema_id": schema_text,
        "digest_bytes": hash_envelope(envelope),
    }
    return reference, payload


def digest_reference_value(reference: dict[str, object]) -> list[PersistenceValue]:
    digest = reference.get("digest_bytes")
    if not isinstance(digest, bytes) or len(digest) != 32:
        raise _error("value_out_of_range", "digest reference must contain 32 bytes")
    return [
        str(reference["envelope_version"]),
        str(reference["algorithm_id"]),
        str(reference["semantic_domain"]),
        str(reference["payload_codec_id"]),
        str(reference["input_schema_id"]),
        digest,
    ]


def _episode_status_value(status: EpisodeStatus) -> list[PersistenceValue]:
    status.to_wire()
    if status.kind == "running":
        return ["running", None]
    assert status.reason is not None
    players: list[PersistenceValue] = []
    for outcome in sorted(status.players, key=lambda item: item.player):
        players.append([outcome.player, outcome.result.value])
    return [status.kind, [status.reason.value, players]]


def calculate_checkpoint_digest_v3(
    full_state_digest: str,
    status: EpisodeStatus,
    counters: dict[str, int],
    codec_id: str,
    semantic_version: str,
) -> str:
    full_state_digest = require_digest(full_state_digest)
    reference: dict[str, object] = {
        "envelope_version": DIGEST_ENVELOPE_ID,
        "algorithm_id": SHA256_ID,
        "semantic_domain": "mtgml.full-state-digest.v3",
        "payload_codec_id": CANONICAL_CBOR_ID,
        "input_schema_id": "full-state-digest-input.v3",
        "digest_bytes": bytes.fromhex(full_state_digest),
    }
    counter_names = (
        "decisions_submitted",
        "accepted_transitions",
        "rule_events_emitted",
        "resource_units_consumed",
        "wall_clock_elapsed_millis",
    )
    counter_values: list[PersistenceValue] = []
    for name in counter_names:
        value = counters.get(name)
        if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 2**64 - 1:
            raise _error("value_out_of_range", f"counter {name} is outside u64")
        counter_values.append(value)
    payload = encode_canonical(
        [
            CHECKPOINT_INPUT_SCHEMA,
            CHECKPOINT_DOMAIN,
            digest_reference_value(reference),
            _episode_status_value(status),
            counter_values,
            [codec_id, semantic_version],
        ]
    )
    return hashlib.sha256(
        encode_envelope(CHECKPOINT_DOMAIN, CHECKPOINT_INPUT_SCHEMA, payload)
    ).hexdigest()
