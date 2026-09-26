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
    if isinstance(value, (list, tuple)):
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

    require(isinstance(value, list) and len(value) == 7, "state record arity")
    require(value[0] == "card-rules-authoritative-state.v1", "state record tag")

    mana = value[1]
    require(isinstance(mana, list), "mana state shape")
    players = []
    for entry in mana:
        require(isinstance(entry, list) and len(entry) == 3, "mana player arity")
        require(isinstance(entry[1], list) and isinstance(entry[2], list), "mana buckets shape")
        require(len(entry[1]) == 6 and len(entry[2]) == 6, "mana bucket arity")
        require(
            all(isinstance(n, int) and 0 <= n <= 0xFFFFFFFF for n in entry[1] + entry[2]),
            "mana count range",
        )
        players.append(entry[0])
    ordered_unique(players, "mana players")

    history = value[2]
    require(isinstance(history, list) and len(history) == 4, "turn history arity")
    require(
        isinstance(history[0], int) and 0 <= history[0] <= 0xFFFFFFFFFFFFFFFF, "turn number range"
    )
    history_players = []
    for entry in history[1]:
        require(isinstance(entry, list) and len(entry) == 7, "player history arity")
        require(isinstance(entry[1], int) and entry[1] in (0, 1), "land play count range")
        require(
            all(
                isinstance(n, int) and 0 <= n <= 0xFFFFFFFF for n in (entry[2], entry[3], entry[5])
            ),
            "turn history count range",
        )
        require(isinstance(entry[4], bool) and isinstance(entry[6], bool), "turn history boolean")
        history_players.append(entry[0])
    ordered_unique(history_players, "history players")
    targets = history[2]
    require(all(isinstance(x, list) and len(x) == 2 for x in targets), "target occurrence arity")
    ordered_unique([tuple(x) for x in targets], "target occurrences")
    used = history[3]
    require(all(isinstance(x, list) and len(x) == 2 for x in used), "once ability arity")
    ordered_unique([tuple(x) for x in used], "once ability entries")

    counters = value[3]
    require(isinstance(counters, list), "counter state shape")
    objects = []
    for entry in counters:
        require(isinstance(entry, list) and len(entry) == 2, "counter object arity")
        objects.append(entry[0])
        tags = []
        for counter in entry[1]:
            require(isinstance(counter, list) and len(counter) == 2, "counter entry arity")
            tag, count = counter
            require(tag in (0, 1, 2), "unknown counter kind")
            require(isinstance(count, int) and 1 <= count <= 0xFFFFFFFF, "counter count range")
            tags.append(tag)
        ordered_unique(tags, "counter kinds")
    ordered_unique(objects, "counter objects")

    attachments = value[4]
    require(isinstance(attachments, list), "attachment state shape")
    sources = []
    for edge in attachments:
        require(isinstance(edge, list) and len(edge) == 4, "attachment edge arity")
        require(
            0 <= edge[2] <= 0xFFFFFFFFFFFFFFFF and 0 <= edge[3] <= 0xFFFFFFFF,
            "attachment timestamp range",
        )
        sources.append(edge[0])
    ordered_unique(sources, "attachment sources")

    faces = value[5]
    require(isinstance(faces, list), "face state shape")
    face_objects = []
    for entry in faces:
        require(isinstance(entry, list) and len(entry) == 2, "face entry arity")
        require(isinstance(entry[1], int) and 0 <= entry[1] <= 0xFFFFFFFF, "face key range")
        face_objects.append(entry[0])
    ordered_unique(face_objects, "face objects")

    abilities = value[6]
    require(isinstance(abilities, list), "ability authority shape")
    ids, semantic_keys = [], []
    for entry in abilities:
        require(isinstance(entry, list) and len(entry) == 3, "ability authority arity")
        require(isinstance(entry[2], int) and 0 <= entry[2] <= 0xFFFFFFFF, "ability key range")
        ids.append(entry[0])
        semantic_keys.append((entry[1], entry[2]))
    ordered_unique(ids, "ability instance IDs")
    ordered_unique(semantic_keys, "ability semantic keys")


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
            and chars[0].lower() == subtype
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
