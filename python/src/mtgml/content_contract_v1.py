"""Closed mechanical mirror of the ContentContractManifestV1 Card IR decoder.

This module validates canonical CBOR structure and the existing Card IR shape;
it does not authorize gameplay or derive rules behavior.
"""

from __future__ import annotations

import unicodedata
from itertools import pairwise
from typing import cast

from .errors import WireError
from .persistence import decode_canonical, encode_canonical


def _fail(message: str) -> None:
    raise WireError("semantic.replay_manifest", message)


def _array(value: object, length: int | None = None) -> list[object]:
    if not isinstance(value, list) or (length is not None and len(value) != length):
        _fail("content manifest array shape is invalid")
    return cast(list[object], value)


def _u64(value: object) -> int:
    if type(value) is not int or not 0 <= value <= 2**64 - 1:
        _fail("content unsigned integer is outside u64")
    return cast(int, value)


def _u32(value: object) -> int:
    if type(value) is not int or not 0 <= value <= 2**32 - 1:
        _fail("content unsigned integer is outside u32")
    return cast(int, value)


def _i32(value: object) -> int:
    if type(value) is not int or not -(2**31) <= value <= 2**31 - 1:
        _fail("content signed integer is outside i32")
    return cast(int, value)


def _text(value: object) -> str:
    if not isinstance(value, str):
        _fail("content text field has wrong type")
    return cast(str, value)


def _valid_text(value: str) -> bool:
    return bool(value) and not any(unicodedata.category(ch) == "Cc" for ch in value)


def _color(value: object) -> str:
    color = _text(value)
    if color not in {"white", "blue", "black", "red", "green"}:
        _fail("unknown mana color")
    return color


def _valid_capability_key(value: str) -> bool:
    segments = value.split("/")

    def word(segment: str) -> bool:
        first = segment[0] if segment else ""
        return (
            bool(segment)
            and first.isascii()
            and (first.islower() or first.isdigit())
            and all(
                char.isascii() and (char.islower() or char.isdigit() or char == "-")
                for char in segment[1:]
            )
        )

    if segments[0] in {"rules", "mechanic", "decision", "visibility", "tooling"}:
        return len(segments) > 1 and all(word(part) for part in segments[1:])
    if len(segments) >= 3 and segments[0] == "format":
        namespace = segments[1]
        return (
            bool(namespace)
            and all(
                char.isascii() and (char.islower() or char.isdigit() or char == "-")
                for char in namespace
            )
            and all(word(part) for part in segments[2:])
        )
    return False


def _valid_capability_version(value: str) -> bool:
    parts = value.split(".")
    return len(parts) == 3 and all(part and part.isascii() and part.isdigit() for part in parts)


def _validate_mana_cost(value: object) -> None:
    if value is None:
        return
    for symbol in _array(value):
        variant, payload = _array(symbol, 2)
        variant = _text(variant)
        if variant == "generic":
            amount = _u32(payload)
            if amount == 0:
                _fail("generic mana symbol must be positive")
        elif variant in {"white", "blue", "black", "red", "green", "colorless"}:
            if payload is not None:
                _fail("colored mana symbol payload must be null")
        elif variant == "hybrid":
            first, second = _array(payload, 2)
            if _color(first) == _color(second):
                _fail("hybrid mana colors must differ")
        else:
            _fail("unknown printed mana symbol")


def _validate_characteristics(value: object) -> list[object]:
    name, mana_cost, color_indicator, type_line, power_toughness, loyalty, defense = _array(
        value, 7
    )
    name = _text(name)
    if not _valid_text(name):
        _fail("card characteristic name is invalid")
    _validate_mana_cost(mana_cost)

    colors = [_color(color) for color in _array(color_indicator)]
    encoded_colors = [encode_canonical(color) for color in colors]
    if any(left >= right for left, right in pairwise(encoded_colors)):
        _fail("color indicator is duplicated or noncanonical")

    type_fields = _array(type_line, 3)
    normalized_type_fields: list[list[str]] = []
    for field in type_fields:
        terms = [_text(term) for term in _array(field)]
        if any(not _valid_text(term) for term in terms) or len(set(terms)) != len(terms):
            _fail("type-line terms are invalid or duplicated")
        normalized_type_fields.append(terms)

    if power_toughness is not None:
        power, toughness = _array(power_toughness, 2)
        _i32(power)
        _i32(toughness)
    if loyalty is not None:
        _i32(loyalty)
    if defense is not None:
        _i32(defense)
    return cast(list[object], normalized_type_fields)


def _validate_profile(binding: object, faces: list[object], abilities: list[object]) -> None:
    tag, profile = _array(binding, 2)
    if _text(tag) == "unprofiled":
        if profile is not None:
            _fail("unprofiled binding payload must be null")
        return
    if tag != "profiled":
        _fail("unknown card semantic binding")

    profile_id, body = _array(profile, 2)
    if _text(profile_id) != "basic-land@1.0.0":
        _fail("unknown card semantic profile")
    body_tag, raw_subtype = _array(body, 2)
    subtype = _text(raw_subtype)
    if _text(body_tag) != "basic-land-profile.v1" or subtype not in {"mountain", "plains"}:
        _fail("unknown basic-land profile body")
    if len(faces) != 1 or _u32(_array(faces[0], 2)[0]) != 0:
        _fail("basic-land profile requires exactly FaceKey(0)")
    characteristics = _array(faces[0], 2)[1]
    expected = [["Basic"], ["Land"], [subtype.title()]]
    if _validate_characteristics(characteristics) != expected:
        _fail("basic-land profile and type line differ")
    if abilities != [[0, 0]]:
        _fail("basic-land profile requires exactly ability identity (0,0)")


def _validate_references(value: object) -> None:
    previous: tuple[int, int, str] | None = None
    for raw in _array(value):
        relation, target, target_face = _array(raw, 3)
        relation = _text(relation)
        target = _u64(target)
        face = None if target_face is None else _u32(target_face)
        if relation != "required_definition":
            _fail("unknown definition reference relation")
        key = (target, -1 if face is None else face, relation)
        if previous is not None and previous >= key:
            _fail("definition references are not strictly ordered")
        previous = key


def _validate_requirements(value: object) -> None:
    previous: tuple[str, str] | None = None
    seen_keys: set[str] = set()
    for raw in _array(value):
        key, version = _array(raw, 2)
        key = _text(key)
        version = _text(version)
        pair = (key, version)
        if (
            not _valid_capability_key(key)
            or not _valid_capability_version(version)
            or (previous is not None and previous >= pair)
            or key in seen_keys
        ):
            _fail("capability requirements are invalid or noncanonical")
        previous = pair
        seen_keys.add(key)


def _validate_definition(value: object) -> int:
    tag, definition_id, raw_faces, raw_abilities, binding, references, requirements = _array(
        value, 7
    )
    if _text(tag) != "card-definition-envelope.v1":
        _fail("unknown card-definition envelope")
    definition_id = _u64(definition_id)

    faces = _array(raw_faces)
    if not faces:
        _fail("card definition must contain a face")
    for index, raw_face in enumerate(faces):
        face_key, characteristics = _array(raw_face, 2)
        if _u32(face_key) != index:
            _fail("face keys are not canonical")
        _validate_characteristics(characteristics)

    abilities = _array(raw_abilities)
    previous_ability: tuple[int, int] | None = None
    seen_ability_keys: set[int] = set()
    for raw_ability in abilities:
        ability_key, face_key = _array(raw_ability, 2)
        ability_key = _u32(ability_key)
        face_key = _u32(face_key)
        if face_key >= len(faces) or ability_key in seen_ability_keys:
            _fail("ability identity is not a valid local reference")
        key = (face_key, ability_key)
        if previous_ability is not None and previous_ability >= key:
            _fail("ability identities are not canonical")
        previous_ability = key
        seen_ability_keys.add(ability_key)

    _validate_profile(binding, faces, abilities)
    _validate_references(references)
    _validate_requirements(requirements)
    return definition_id


def decode_content_contract_manifest_v1(payload: bytes) -> bytes:
    """Validate closed V1 Card IR content CBOR and return its canonical bytes."""
    try:
        value = decode_canonical(payload)
        tag, identity, raw_definitions = _array(value, 3)
        if _text(tag) != "content-contract-manifest.v1" or _text(identity) != (
            "mtgml.content-contract.v1"
        ):
            _fail("unknown content-contract manifest identity")
        definitions = [_validate_definition(row) for row in _array(raw_definitions)]
        if any(left >= right for left, right in pairwise(definitions)):
            _fail("content definitions are duplicated or noncanonical")
        canonical = encode_canonical(value)
        if canonical != payload:
            _fail("content manifest CBOR is noncanonical")
        return canonical
    except WireError:
        raise
    except (TypeError, ValueError, OverflowError) as exc:
        raise WireError("semantic.replay_manifest", "content manifest is invalid") from exc
