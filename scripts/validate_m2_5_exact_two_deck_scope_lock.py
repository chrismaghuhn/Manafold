#!/usr/bin/env python3
"""Validate the source-bound M2.5 exact two-deck scope lock."""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import os
import sys
import zipfile
from pathlib import Path
from typing import Any

sys.dont_write_bytecode = True
import jsonschema

ROOT = Path(__file__).resolve().parents[1]
LOCK_RELATIVE_PATH = Path("sources/m2_5/scope/exact_two_deck_scope_lock.v1.json")
SCHEMA_RELATIVE_PATH = Path("schemas/m2-5-exact-two-deck-scope-lock.v1.schema.json")
ARCHIVE_RELATIVE_PATH = Path("m2_5/Manafold_M2_5_Pre_Research_ALL_ARTIFACTS_REV3.zip")
ARCHIVE_ENV_VAR = "MANAFOLD_SOURCE_ARCHIVE"
EXPECTED_SOURCE_PACKAGE_SHA256 = "99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90"
EXPECTED_SCHEMA = "manafold.m2.5.exact-two-deck-scope-lock.v1"
PACKAGE_MANIFEST_MEMBER = "Manafold_M2_5_Package_Manifest_REV3.json"
EXPECTED_DECKS = (
    (1, "m2-5/token-triumph", "Token Triumph"),
    (2, "m2-5/grave-danger", "Grave Danger"),
)


class ScopeLockValidationError(ValueError):
    """The lock is malformed, inconsistent, or not source-bound."""


def load_lock(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ScopeLockValidationError("scope lock must be a JSON object")
    return value


def _canonical_json_bytes(value: object) -> bytes:
    return (
        json.dumps(
            value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False
        )
        + "\n"
    ).encode("utf-8")


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ScopeLockValidationError(message)


def _deck_content_payload(deck: dict[str, Any]) -> dict[str, Any]:
    return {
        "card_count": deck["card_count"],
        "cards": deck["cards"],
        "commander": deck["commander"],
        "deck_id": deck["deck_id"],
        "deck_name": deck["deck_name"],
        "oracle_source_identities": deck["oracle_source_identities"],
        "player": deck["player"],
        "sideboard": deck["sideboard"],
        "source_snapshot": deck["source_snapshot"],
    }


def compute_deck_content_sha256(deck: dict[str, Any]) -> str:
    """Hash the canonical UTF-8 JSON payload without the stored digest itself."""

    return _sha256_bytes(_canonical_json_bytes(_deck_content_payload(deck)))


def _validate_source_identity_set(deck: dict[str, Any]) -> None:
    rows = deck["cards"]
    expected = [
        {
            "normalized_record_sha256": row["oracle_normalized_record_sha256"],
            "oracle_semantic_identity": row["oracle_semantic_identity"],
            "source_record_id": row["oracle_source_record_id"],
            "source_record_raw_sha256": row["oracle_source_record_raw_sha256"],
        }
        for row in rows
    ]
    expected.sort(key=lambda item: item["oracle_semantic_identity"])
    actual = deck["oracle_source_identities"]
    _require(
        actual == expected, f"{deck['deck_name']} Oracle/source identity set differs from rows"
    )
    _require(
        len({item["oracle_semantic_identity"] for item in actual}) == len(actual),
        f"{deck['deck_name']} Oracle identity set contains duplicates",
    )


def _validate_deck(deck: dict[str, Any], expected: tuple[int, str, str]) -> None:
    player, deck_id, deck_name = expected
    _require(deck["player"] == player, f"unexpected player seat for {deck_name}")
    _require(deck["deck_id"] == deck_id, f"unexpected deck_id for {deck_name}")
    _require(deck["deck_name"] == deck_name, f"unexpected deck name: {deck['deck_name']!r}")
    _require(deck["sideboard"] == [], f"{deck_name} has a sideboard")
    rows = deck["cards"]
    _require(
        len({row["source_row_id"] for row in rows}) == len(rows),
        f"{deck_name} has a duplicate source row",
    )
    _require(
        len({row["source_line_number"] for row in rows}) == len(rows),
        f"{deck_name} has a duplicate source line",
    )
    _require(
        [row["source_line_number"] for row in rows]
        == sorted(row["source_line_number"] for row in rows),
        f"{deck_name} rows are not in source order",
    )
    _require(sum(row["quantity"] for row in rows) == 100, f"{deck_name} quantity total is not 100")
    _require(deck["card_count"] == 100, f"{deck_name} card_count is not 100")
    commander_rows = [row for row in rows if row["is_commander"]]
    _require(len(commander_rows) == 1, f"{deck_name} must have exactly one commander row")
    commander_row = commander_rows[0]
    commander = deck["commander"]
    for field, row_field in (
        ("card_name", "card_name"),
        ("oracle_semantic_identity", "oracle_semantic_identity"),
        ("oracle_source_record_id", "oracle_source_record_id"),
        ("source_row_id", "source_row_id"),
    ):
        _require(
            commander[field] == commander_row[row_field],
            f"{deck_name} commander {field} differs from its row",
        )
    _require(
        commander["color_identity"] == commander_row["oracle_color_identity"],
        f"{deck_name} commander color identity differs from its row",
    )
    _require(commander["oracle_is_legendary"], f"{deck_name} commander is not legendary")
    commander_colors = set(commander["color_identity"])
    _require(
        all(set(row["oracle_color_identity"]).issubset(commander_colors) for row in rows),
        f"{deck_name} contains a card outside commander color identity",
    )
    names = {}
    for row in rows:
        names[row["card_name"]] = names.get(row["card_name"], 0) + row["quantity"]
        _require(
            row["oracle_commander_legality"] == "legal",
            f"{deck_name} has a card not legal under the Oracle Commander flag: {row['card_name']}",
        )
    for row in rows:
        if not row["is_basic_land"]:
            _require(
                names[row["card_name"]] == 1, f"{deck_name} repeats non-basic {row['card_name']}"
            )
    _validate_source_identity_set(deck)
    _require(
        deck["content_sha256"] == compute_deck_content_sha256(deck),
        f"{deck_name} content digest does not recompute",
    )


def validate_lock_document(lock: dict[str, Any], *, schema_path: Path | None = None) -> None:
    _require(
        isinstance(lock.get("decks"), list) and len(lock["decks"]) == 2,
        "scope lock must contain exactly two decks",
    )
    schema = json.loads((schema_path or ROOT / SCHEMA_RELATIVE_PATH).read_text(encoding="utf-8"))
    try:
        jsonschema.Draft202012Validator(schema).validate(lock)
    except jsonschema.ValidationError as exc:
        raise ScopeLockValidationError(
            f"scope lock schema validation failed: {exc.message}"
        ) from exc
    _require(lock["schema"] == EXPECTED_SCHEMA, "wrong scope lock schema")
    _require(
        lock["deck_count"] == len(lock["decks"]) == 2, "scope lock must contain exactly two decks"
    )
    _require(lock["sideboards"] == [], "scope lock sideboards must be empty")
    _require(
        lock["source_package"]["sha256"] == EXPECTED_SOURCE_PACKAGE_SHA256,
        "scope lock source package is not the pinned REV3 package",
    )
    _require(
        [deck["player"] for deck in lock["decks"]] == [1, 2],
        "scope lock players must be exactly 1 then 2",
    )
    for deck, expected in zip(lock["decks"], EXPECTED_DECKS, strict=True):
        _validate_deck(deck, expected)


def validate_scope_matrix(lock: dict[str, Any], path: Path | None = None) -> None:
    """Require the normative scope matrix to repeat the machine-readable lock."""

    matrix = (path or ROOT / "docs/contracts/V1_SCOPE_MATRIX.md").read_text(encoding="utf-8")
    required_fragments = (
        "sources/m2_5/scope/exact_two_deck_scope_lock.v1.json",
        "EXACT_TWO_DECK_SELECTION       = PASS",
        "DECK_PAIR_LOCKED               = YES",
        "PLAYER_1                      = Token Triumph",
        "PLAYER_2                      = Grave Danger",
        f"{lock['decks'][0]['content_sha256']}",
        f"{lock['decks'][1]['content_sha256']}",
        "AUTHORITATIVE_RANKING_AVAILABLE = NO",
        "C_PASS                         = BLOCKED",
        "M2_5_FINAL                     = NOT_YET",
        "M3                            = NOT_AUTHORIZED",
    )
    for fragment in required_fragments:
        _require(fragment in matrix, f"scope matrix does not repeat lock field: {fragment}")


def _archive_path(archive_root: Path) -> Path:
    root = archive_root.resolve()
    path = (root / ARCHIVE_RELATIVE_PATH).resolve()
    try:
        path.relative_to(root)
    except ValueError as exc:
        raise ScopeLockValidationError("configured archive path escapes archive root") from exc
    return path


def _member_bytes(archive: zipfile.ZipFile, member: str) -> bytes:
    try:
        return archive.read(member)
    except KeyError as exc:
        raise ScopeLockValidationError(f"required archive member is missing: {member}") from exc


def _member_binding(lock: dict[str, Any], member: str) -> dict[str, Any]:
    for section in (
        lock["source_bindings"],
        lock["snapshot_identities"],
    ):
        values = section.values()
        for value in values:
            candidates = value if isinstance(value, list) else [value]
            for item in candidates:
                if item.get("archive_member") == member:
                    return item
    for deck in lock["decks"]:
        if deck["source_snapshot"]["archive_member"] == member:
            return deck["source_snapshot"]
    raise ScopeLockValidationError(f"lock has no binding for archive member: {member}")


def _verify_member(archive: zipfile.ZipFile, binding: dict[str, Any]) -> bytes:
    raw = _member_bytes(archive, binding["archive_member"])
    _require(
        len(raw) == binding["bytes"],
        f"archive member byte count mismatch: {binding['archive_member']}",
    )
    _require(
        _sha256_bytes(raw) == binding["sha256"],
        f"archive member digest mismatch: {binding['archive_member']}",
    )
    return raw


def _verify_manifest_bindings(lock: dict[str, Any], archive: zipfile.ZipFile) -> None:
    manifest_raw = _member_bytes(archive, PACKAGE_MANIFEST_MEMBER)
    package = json.loads(manifest_raw.decode("utf-8"))
    _require(
        package["schema"] == "manafold.m2.5.rev3.package-manifest.v1",
        "wrong REV3 package manifest schema",
    )
    _require(
        _sha256_bytes(manifest_raw) == lock["source_package"]["manifest_member_sha256"],
        "REV3 package manifest digest mismatch",
    )
    entries = {entry["path"]: entry for entry in package["entries"]}
    referenced = set()
    for section in (lock["source_bindings"], lock["snapshot_identities"]):
        values = section.values()
        for value in values:
            candidates = value if isinstance(value, list) else [value]
            for binding in candidates:
                path = binding["archive_member"]
                if path in entries:
                    referenced.add(path)
                    _require(
                        entries[path]["sha256"] == binding["sha256"],
                        f"lock digest differs from package manifest for {path}",
                    )
    for deck in lock["decks"]:
        path = deck["source_snapshot"]["archive_member"]
        referenced.add(path)
        _require(
            entries[path]["sha256"] == deck["source_snapshot"]["sha256"],
            f"lock digest differs from package manifest for {path}",
        )
    _require(referenced, "lock has no package-bound members")


def _parse_oracle_records(raw: bytes) -> tuple[dict[str, dict[str, Any]], dict[str, str]]:
    records: dict[str, dict[str, Any]] = {}
    raw_digests: dict[str, str] = {}
    for line in raw.splitlines(keepends=True):
        record = json.loads(line)
        records[record["id"]] = record
        raw_digests[record["id"]] = _sha256_bytes(line)
    return records, raw_digests


def _rows_by_deck(raw: bytes) -> dict[str, list[dict[str, str]]]:
    rows = list(csv.DictReader(io.StringIO(raw.decode("utf-8"), newline="")))
    selected: dict[str, list[dict[str, str]]] = {name: [] for _, _, name in EXPECTED_DECKS}
    for row in rows:
        if row["deck_id"] in selected:
            selected[row["deck_id"]].append(row)
    return selected


def _parse_source_index(raw: bytes) -> dict[str, dict[str, str]]:
    rows = list(csv.DictReader(io.StringIO(raw.decode("utf-8"), newline="")))
    oracle_rows = [row for row in rows if row["bulk_type"] == "oracle_cards"]
    by_record_id = {row["source_record_id"]: row for row in oracle_rows}
    _require(
        len(by_record_id) == len(oracle_rows),
        "Oracle source record index contains duplicate record IDs",
    )
    return by_record_id


def _verify_deck_source(
    lock_deck: dict[str, Any],
    snapshot_raw: bytes,
    csv_rows: list[dict[str, str]],
    oracle: dict[str, dict[str, Any]],
    oracle_raw_digests: dict[str, str],
    source_index: dict[str, dict[str, str]],
) -> None:
    expected_rows = lock_deck["cards"]
    snapshot_lines = snapshot_raw.decode("utf-8").splitlines()
    _require(
        snapshot_lines[0] == f"// NAME: {lock_deck['deck_name']}",
        f"{lock_deck['deck_name']} source name provenance mismatch",
    )
    _require(
        snapshot_lines[1].startswith("// SOURCE: "),
        f"{lock_deck['deck_name']} source URL provenance is missing",
    )
    _require(
        snapshot_lines[2] == f"// DATE: {lock_deck['source_snapshot']['source_date']}",
        f"{lock_deck['deck_name']} source date provenance mismatch",
    )
    _require(
        snapshot_lines[1][len("// SOURCE: ") :] == lock_deck["source_snapshot"]["source_url"],
        f"{lock_deck['deck_name']} source URL provenance mismatch",
    )
    _require(
        len(csv_rows) == len(expected_rows), f"{lock_deck['deck_name']} source row count differs"
    )
    by_id = {row["deck_row_id"]: row for row in csv_rows}
    _require(len(by_id) == len(csv_rows), f"{lock_deck['deck_name']} source CSV has duplicate rows")
    for expected in expected_rows:
        actual = by_id.get(expected["source_row_id"])
        _require(actual is not None, f"missing source row: {expected['source_row_id']}")
        _require(
            snapshot_lines[expected["source_line_number"] - 1] == expected["source_line_text"],
            f"source snapshot line mismatch for {expected['source_row_id']}",
        )
        for field, expected_value in (
            ("source_snapshot_file", expected["source_snapshot_file"]),
            ("source_line_number", str(expected["source_line_number"])),
            ("source_line_text", expected["source_line_text"]),
            ("quantity", str(expected["quantity"])),
            ("is_commander", str(expected["is_commander"])),
            ("card_name", expected["card_name"]),
            ("oracle_semantic_identity", expected["oracle_semantic_identity"]),
            ("oracle_source_record_id", expected["oracle_source_record_id"]),
            ("oracle_source_record_raw_sha256", expected["oracle_source_record_raw_sha256"]),
            ("oracle_normalized_record_sha256", expected["oracle_normalized_record_sha256"]),
            ("oracle_source_full_offset", str(expected["oracle_source_full_offset"])),
            ("oracle_source_full_length", str(expected["oracle_source_full_length"])),
        ):
            _require(
                actual[field] == expected_value,
                f"source row mismatch for {expected['source_row_id']}: {field}",
            )
        record = oracle.get(expected["oracle_source_record_id"])
        _require(
            record is not None,
            f"missing Oracle source record: {expected['oracle_source_record_id']}",
        )
        _require(
            oracle_raw_digests.get(expected["oracle_source_record_id"])
            == expected["oracle_source_record_raw_sha256"],
            f"Oracle raw-record digest mismatch for {expected['card_name']}",
        )
        indexed = source_index.get(expected["oracle_source_record_id"])
        _require(
            indexed is not None,
            f"missing source-record index row: {expected['oracle_source_record_id']}",
        )
        for field, expected_value in (
            ("oracle_semantic_identity", expected["oracle_semantic_identity"]),
            ("full_uncompressed_byte_offset", str(expected["oracle_source_full_offset"])),
            ("full_uncompressed_byte_length", str(expected["oracle_source_full_length"])),
            ("source_record_raw_sha256", expected["oracle_source_record_raw_sha256"]),
            ("normalized_record_sha256", expected["oracle_normalized_record_sha256"]),
        ):
            _require(
                indexed[field] == expected_value,
                f"source-record index mismatch for {expected['card_name']}: {field}",
            )
        _require(
            record["name"] == expected["card_name"],
            f"Oracle name mismatch for {expected['card_name']}",
        )
        _require(
            record.get("color_identity", []) == expected["oracle_color_identity"],
            f"Oracle color identity mismatch for {expected['card_name']}",
        )
        _require(
            ("Basic Land" in record.get("type_line", "")) == expected["is_basic_land"],
            f"basic-land classification mismatch for {expected['card_name']}",
        )
        _require(
            record.get("legalities", {}).get("commander") == expected["oracle_commander_legality"],
            f"Commander legality mismatch for {expected['card_name']}",
        )
        _require(
            record.get("oracle_id") == expected["oracle_semantic_identity"],
            f"Oracle semantic identity mismatch for {expected['card_name']}",
        )
        _require(
            record.get("type_line", "").startswith("Legendary") == expected["oracle_is_legendary"],
            f"legendary classification mismatch for {expected['card_name']}",
        )


def verify_pinned_archive(lock: dict[str, Any], archive_root: Path) -> None:
    path = _archive_path(archive_root)
    _require(path.is_file(), f"{ARCHIVE_ENV_VAR} archive is missing: {path}")
    raw_archive = path.read_bytes()
    _require(
        _sha256_bytes(raw_archive) == EXPECTED_SOURCE_PACKAGE_SHA256,
        "REV3 source package digest mismatch",
    )
    with zipfile.ZipFile(io.BytesIO(raw_archive)) as archive:
        _verify_manifest_bindings(lock, archive)
        for binding in lock["source_bindings"].values():
            _verify_member(archive, binding)
        for value in lock["snapshot_identities"].values():
            values = value if isinstance(value, list) else [value]
            for binding in values:
                _verify_member(archive, binding)
        snapshot_bytes_by_name = {}
        for deck in lock["decks"]:
            snapshot = deck["source_snapshot"]
            snapshot_raw = _verify_member(archive, snapshot)
            snapshot_bytes_by_name[deck["deck_name"]] = snapshot_raw
            _require(
                b"SIDEBOARD" not in snapshot_raw.upper(),
                f"{deck['deck_name']} source snapshot contains a sideboard",
            )
        resolution = _rows_by_deck(
            _verify_member(archive, lock["source_bindings"]["deck_row_source_resolution"])
        )
        oracle, oracle_raw_digests = _parse_oracle_records(
            _verify_member(archive, lock["source_bindings"]["oracle_source_records"])
        )
        source_index = _parse_source_index(
            _verify_member(archive, lock["source_bindings"]["source_record_index"])
        )
        for deck in lock["decks"]:
            _verify_deck_source(
                deck,
                snapshot_bytes_by_name[deck["deck_name"]],
                resolution[deck["deck_name"]],
                oracle,
                oracle_raw_digests,
                source_index,
            )
        policy = json.loads(
            _verify_member(archive, lock["snapshot_identities"]["legality"]).decode("utf-8")
        )
        banned = set(policy["commander_banned_names"])
        for deck in lock["decks"]:
            names = {row["card_name"] for row in deck["cards"]}
            _require(not names & banned, f"{deck['deck_name']} contains a Commander-banned card")
            commander_colors = set(deck["commander"]["color_identity"])
            _require(
                all(
                    set(row["oracle_color_identity"]).issubset(commander_colors)
                    for row in deck["cards"]
                ),
                f"{deck['deck_name']} violates Commander color identity",
            )
    print("PASS: exact two-deck scope lock and pinned REV3 source binding verified")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--lock", type=Path, default=ROOT / LOCK_RELATIVE_PATH)
    parser.add_argument("--archive-root", type=Path)
    args = parser.parse_args()
    try:
        lock = load_lock(args.lock)
        validate_lock_document(lock)
        validate_scope_matrix(lock)
        configured_root = args.archive_root
        if configured_root is None:
            configured = os.environ.get(ARCHIVE_ENV_VAR)
            if not configured:
                print(f"BLOCKED: {ARCHIVE_ENV_VAR} is not configured")
                return 2
            configured_root = Path(configured)
        verify_pinned_archive(lock, configured_root)
    except (OSError, ValueError, KeyError, json.JSONDecodeError, zipfile.BadZipFile) as exc:
        print(f"FAIL: {exc}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
