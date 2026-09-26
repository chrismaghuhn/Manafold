"""Test-only mechanical reference for accepted M4 successor byte contracts.

This module is intentionally detached from production packages. It encodes
only CBOR primitives and the already frozen positional arrays used by the
Phase-2 vectors; it contains no Magic legality or transition logic.
"""

from __future__ import annotations

import base64
import hashlib
from typing import Any

ENVELOPE_ID = b"mtgml.digest-envelope.v1\x00"


def _head(major: int, value: int) -> bytes:
    if value < 0:
        raise ValueError("CBOR head argument must be nonnegative")
    prefix = major << 5
    if value < 24:
        return bytes([prefix | value])
    if value <= 0xFF:
        return bytes([prefix | 24, value])
    if value <= 0xFFFF:
        return bytes([prefix | 25]) + value.to_bytes(2, "big")
    if value <= 0xFFFFFFFF:
        return bytes([prefix | 26]) + value.to_bytes(4, "big")
    if value <= 0xFFFFFFFFFFFFFFFF:
        return bytes([prefix | 27]) + value.to_bytes(8, "big")
    raise ValueError("integer outside CBOR u64 range")


def encode(value: Any) -> bytes:
    """Encode the closed primitive subset used by the frozen vectors."""

    if value is None:
        return b"\xf6"
    if value is False:
        return b"\xf4"
    if value is True:
        return b"\xf5"
    if isinstance(value, int):
        return _head(0, value) if value >= 0 else _head(1, -1 - value)
    if isinstance(value, bytes):
        return _head(2, len(value)) + value
    if isinstance(value, str):
        raw = value.encode("utf-8")
        return _head(3, len(raw)) + raw
    if isinstance(value, list | tuple):
        return _head(4, len(value)) + b"".join(encode(item) for item in value)
    raise TypeError(f"unsupported reference-CBOR value: {type(value).__name__}")


def decode(data: bytes) -> Any:
    """Decode the primitive subset in the frozen V5 baseline fixture."""

    def argument(offset: int, additional: int) -> tuple[int, int]:
        if additional < 24:
            return additional, offset
        widths = {24: 1, 25: 2, 26: 4, 27: 8}
        width = widths.get(additional)
        if width is None or offset + width > len(data):
            raise ValueError("invalid or truncated canonical CBOR argument")
        value = int.from_bytes(data[offset : offset + width], "big")
        if value < (24 if width == 1 else 1 << (8 * (width // 2))):
            raise ValueError("non-shortest CBOR argument")
        return value, offset + width

    def item(offset: int) -> tuple[Any, int]:
        if offset >= len(data):
            raise ValueError("truncated CBOR item")
        initial = data[offset]
        offset += 1
        major, additional = initial >> 5, initial & 31
        if major == 7:
            if initial == 0xF4:
                return False, offset
            if initial == 0xF5:
                return True, offset
            if initial == 0xF6:
                return None, offset
            raise ValueError("unsupported CBOR simple value")
        length, offset = argument(offset, additional)
        if major == 0:
            return length, offset
        if major == 1:
            return -1 - length, offset
        if major in (2, 3):
            end = offset + length
            if end > len(data):
                raise ValueError("truncated CBOR string")
            raw = data[offset:end]
            return (raw if major == 2 else raw.decode("utf-8")), end
        if major == 4:
            values = []
            for _ in range(length):
                child, offset = item(offset)
                values.append(child)
            return values, offset
        raise ValueError("unsupported CBOR major type")

    result, end = item(0)
    if end != len(data):
        raise ValueError("trailing CBOR value")
    return result


def digest_envelope(domain: str, schema: str, payload: bytes) -> tuple[bytes, str]:
    """Return the exact framed digest envelope and lowercase SHA-256."""

    fields = (
        b"sha-256",
        domain.encode("ascii"),
        b"mtgml.canonical-cbor.v1",
        schema.encode("ascii"),
        payload,
    )
    envelope = ENVELOPE_ID + b"".join(len(field).to_bytes(8, "big") + field for field in fields)
    return envelope, hashlib.sha256(envelope).hexdigest()


def validate_authoritative_state(value: Any) -> None:
    """Validate only the closed canonical shape from State Spec §16."""

    def require(condition: bool, message: str) -> None:
        if not condition:
            raise ValueError(message)

    def ordered_unique(keys: list[Any], name: str) -> None:
        require(keys == sorted(keys), f"{name} is not canonically ordered")
        require(len(keys) == len(set(keys)), f"{name} contains duplicates")

    def require_u32(value: Any, name: str, *, minimum: int = 0) -> None:
        # bool subclasses int in Python, but the wire contract distinguishes
        # CBOR booleans from unsigned integer values.
        require(type(value) is int and minimum <= value <= 0xFFFFFFFF, f"{name} u32 range")

    def require_u64(value: Any, name: str) -> None:
        require(type(value) is int and 0 <= value <= 0xFFFFFFFFFFFFFFFF, f"{name} u64 range")

    require(isinstance(value, list) and len(value) == 7, "state record arity")
    require(value[0] == "card-rules-authoritative-state.v1", "state record tag")

    mana = value[1]
    require(isinstance(mana, list), "mana state shape")
    players = []
    for entry in mana:
        require(isinstance(entry, list) and len(entry) == 3, "mana player arity")
        require_u64(entry[0], "mana player ID")
        require(isinstance(entry[1], list) and isinstance(entry[2], list), "mana buckets shape")
        require(len(entry[1]) == 6 and len(entry[2]) == 6, "mana bucket arity")
        for n in entry[1] + entry[2]:
            require_u32(n, "mana count")
        players.append(entry[0])
    ordered_unique(players, "mana players")

    history = value[2]
    require(isinstance(history, list) and len(history) == 4, "turn history arity")
    require_u64(history[0], "turn number")
    history_players = []
    for entry in history[1]:
        require(isinstance(entry, list) and len(entry) == 7, "player history arity")
        require_u64(entry[0], "history player ID")
        require(
            type(entry[1]) is int and entry[1] in (0, 1), "land play count must be 0 or 1"
        )
        for n in (entry[2], entry[3], entry[5]):
            require_u32(n, "turn history count")
        require(isinstance(entry[4], bool) and isinstance(entry[6], bool), "turn history boolean")
        history_players.append(entry[0])
    ordered_unique(history_players, "history players")
    targets = history[2]
    require(all(isinstance(x, list) and len(x) == 2 for x in targets), "target occurrence arity")
    for target, controller in targets:
        require_u64(target, "target object ID")
        require_u64(controller, "targeting controller ID")
    ordered_unique([tuple(x) for x in targets], "target occurrences")
    used = history[3]
    require(all(isinstance(x, list) and len(x) == 2 for x in used), "once ability arity")
    for source, ability_key in used:
        require_u64(source, "once ability source ID")
        require_u32(ability_key, "once ability key")
    ordered_unique([tuple(x) for x in used], "once ability entries")

    counters = value[3]
    require(isinstance(counters, list), "counter state shape")
    objects = []
    for entry in counters:
        require(isinstance(entry, list) and len(entry) == 2, "counter object arity")
        require_u64(entry[0], "counter object ID")
        objects.append(entry[0])
        tags = []
        for counter in entry[1]:
            require(isinstance(counter, list) and len(counter) == 2, "counter entry arity")
            tag, count = counter
            require(type(tag) is int and tag in (0, 1, 2), "unknown counter kind")
            require_u32(count, "counter count", minimum=1)
            tags.append(tag)
        ordered_unique(tags, "counter kinds")
    ordered_unique(objects, "counter objects")

    attachments = value[4]
    require(isinstance(attachments, list), "attachment state shape")
    sources = []
    for edge in attachments:
        require(isinstance(edge, list) and len(edge) == 4, "attachment edge arity")
        require_u64(edge[0], "attachment source ID")
        require_u64(edge[1], "attachment target ID")
        require_u64(edge[2], "attachment revision")
        require_u32(edge[3], "attachment operation ordinal")
        sources.append(edge[0])
    ordered_unique(sources, "attachment sources")

    faces = value[5]
    require(isinstance(faces, list), "face state shape")
    face_objects = []
    for entry in faces:
        require(isinstance(entry, list) and len(entry) == 2, "face entry arity")
        require_u64(entry[0], "face object ID")
        require_u32(entry[1], "face key")
        face_objects.append(entry[0])
    ordered_unique(face_objects, "face objects")

    abilities = value[6]
    require(isinstance(abilities, list), "ability authority shape")
    ids, semantic_keys = [], []
    for entry in abilities:
        require(isinstance(entry, list) and len(entry) == 3, "ability authority arity")
        require_u64(entry[0], "ability instance ID")
        require_u64(entry[1], "ability source ID")
        require_u32(entry[2], "ability key")
        ids.append(entry[0])
        semantic_keys.append((entry[1], entry[2]))
    ordered_unique(ids, "ability instance IDs")
    require(
        len(semantic_keys) == len(set(semantic_keys)),
        "ability semantic keys contain duplicates",
    )


def validate_basic_land_content_manifest(value: Any) -> bytes:
    """Validate the closed content/profile rows used by the Phase-2 KAT."""

    if not (
        isinstance(value, list)
        and len(value) == 3
        and value[0] == "content-contract-manifest.v1"
        and value[1] == "mtgml.content-contract.v1"
        and isinstance(value[2], list)
    ):
        raise ValueError("content manifest identity/arity")
    ids = []
    expected_subtypes = {"mountain", "plains"}
    for definition in value[2]:
        if not isinstance(definition, list) or len(definition) != 7:
            raise ValueError("definition envelope arity")
        if definition[0] != "card-definition-envelope.v1":
            raise ValueError("definition envelope tag")
        ids.append(definition[1])
        binding = definition[4]
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
            and binding[1][1][1] in expected_subtypes
        ):
            raise ValueError("basic land profile body")
        face = definition[2]
        if not isinstance(face, list) or len(face) != 1 or face[0][0] != 0:
            raise ValueError("basic land face shape")
        chars = face[0][1]
        subtype = binding[1][1][1]
        if not (
            isinstance(chars, list)
            and len(chars) == 7
            and isinstance(chars[0], str)
            and chars[3] == [["Basic"], ["Land"], [subtype.title()]]
        ):
            raise ValueError("profile/type-line mismatch")
        if definition[3] != [[0, 0]] or definition[5:] != [[], []]:
            raise ValueError("basic land identity/extension shape")
    if ids != sorted(ids) or len(ids) != len(set(ids)):
        raise ValueError("definition order/uniqueness")
    return encode(value)


def verify_content_child(child: Any, parent_content_id: str) -> str:
    """Strictly verify the Replay V7 child transport used in the KAT."""

    if not isinstance(child, dict) or set(child) != {
        "content_contract_id",
        "manifest_canonical_cbor_base64",
    }:
        raise ValueError("content child closed fields")
    child_id = child["content_contract_id"]
    encoded = child["manifest_canonical_cbor_base64"]
    if (
        not isinstance(child_id, str)
        or len(child_id) != 64
        or any(c not in "0123456789abcdef" for c in child_id)
    ):
        raise ValueError("content child ID format")
    if child_id != parent_content_id:
        raise ValueError("content parent/child mismatch")
    if not isinstance(encoded, str) or len(encoded) > 89_478_488:
        raise ValueError("content child transport bound")
    try:
        raw = base64.b64decode(encoded, validate=True)
    except (ValueError, base64.binascii.Error) as error:
        raise ValueError("content child Base64") from error
    if base64.b64encode(raw).decode("ascii") != encoded:
        raise ValueError("content child Base64 canonicality")
    manifest = decode(raw)
    canonical = validate_basic_land_content_manifest(manifest)
    if canonical != raw:
        raise ValueError("content child CBOR canonicality")
    _, actual_id = digest_envelope(
        "mtgml.content-contract.v1", "content-contract-manifest.v1", canonical
    )
    if actual_id != child_id:
        raise ValueError("content child digest mismatch")
    return actual_id


def verify_checkpoint_digest_v7(
    value: Any, supplied_digest: str, expected_semantic_contract_id: str
) -> str:
    """Verify the detached V7 digest input and its checkpoint bindings."""

    def require(condition: bool, message: str) -> None:
        if not condition:
            raise ValueError(message)

    require(isinstance(value, list) and len(value) == 7, "checkpoint input arity")
    require(value[0] == "environment-checkpoint-digest-input.v7", "checkpoint input schema")
    require(value[1] == "mtgml.checkpoint-digest.v7", "checkpoint semantic domain")
    state_ref = value[2]
    require(isinstance(state_ref, list) and len(state_ref) == 6, "full-state reference arity")
    require(
        state_ref[:5]
        == [
            "mtgml.digest-envelope.v1",
            "sha-256",
            "mtgml.full-state-digest.v6",
            "mtgml.canonical-cbor.v1",
            "full-state-digest-input.v6",
        ],
        "full-state successor identity",
    )
    require(type(state_ref[5]) is bytes and len(state_ref[5]) == 32, "full-state digest width")
    require(isinstance(value[4], list) and len(value[4]) == 5, "environment counter arity")
    require(
        all(type(counter) is int and 0 <= counter <= 0xFFFFFFFFFFFFFFFF for counter in value[4]),
        "environment counter range",
    )
    require(value[5] == ["in-memory-reference", "7"], "checkpoint codec identity")
    execution_identity = value[6]
    require(
        isinstance(execution_identity, list)
        and len(execution_identity) == 2
        and type(execution_identity[1]) is bytes
        and len(execution_identity[1]) == 32,
        "execution identity shape",
    )
    require(
        isinstance(expected_semantic_contract_id, str)
        and len(expected_semantic_contract_id) == 64
        and all(c in "0123456789abcdef" for c in expected_semantic_contract_id),
        "expected semantic contract ID format",
    )
    require(
        execution_identity[1].hex() == expected_semantic_contract_id,
        "semantic contract identity mismatch",
    )
    require(
        isinstance(supplied_digest, str)
        and len(supplied_digest) == 64
        and all(c in "0123456789abcdef" for c in supplied_digest),
        "checkpoint digest format",
    )
    _, actual_digest = digest_envelope(
        "mtgml.checkpoint-digest.v7",
        "environment-checkpoint-digest-input.v7",
        encode(value),
    )
    require(actual_digest == supplied_digest, "checkpoint digest mismatch")
    return actual_digest
