#!/usr/bin/env python3
"""Build and validate the source-bound M2.5 selected-pair CDI census."""

from __future__ import annotations

import argparse
import copy
import csv
import hashlib
import io
import json
import os
import zipfile
from collections import defaultdict
from pathlib import Path
from typing import Any

import jsonschema
from validate_m2_5_exact_two_deck_scope_lock import (
    EXPECTED_SOURCE_PACKAGE_SHA256,
    load_lock,
    validate_lock_document,
    verify_pinned_archive,
)

ROOT = Path(__file__).resolve().parents[1]
LOCK_RELATIVE_PATH = Path("sources/m2_5/scope/exact_two_deck_scope_lock.v1.json")
SCHEMA_RELATIVE_PATH = Path("schemas/m2-5-selected-pair-cdi-census.v1.schema.json")
SCOPE_RELATIVE_PATH = Path("sources/m2_5/scope")
TASK_ID = "M2_5_SELECTED_PAIR_CDI_CENSUS_01"
SCHEMA_ID = "manafold.m2.5.selected-pair-cdi-census.v1"
ARCHIVE_ENV_VAR = "MANAFOLD_SOURCE_ARCHIVE"

ARTIFACTS = {
    "capability": "selected_pair_capability_census.v1.json",
    "decision": "selected_pair_decision_census.v1.json",
    "information": "selected_pair_information_census.v1.json",
    "generated_object": "selected_pair_generated_object_census.v1.json",
    "recursive_capability_closure": "selected_pair_recursive_capability_closure.v1.json",
}

B2_FILES = {
    "current_root": (
        "sources/m2_5/closures/B2/current_root.json",
        "manafold.m2.5.b2.closure-current-root.v1",
    ),
    "closure_v2": (
        "sources/m2_5/closures/B2/classification_closure.v2.json",
        "manafold.m2.5.b2.classification-closure.v2",
    ),
    "classifications": (
        "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
        "manafold.m2.5.b2.card-semantic-classifications.v1",
    ),
    "projection": (
        "sources/m2_5/closures/B2/deck_row_classification_refs.v1.csv",
        "manafold.m2.5.b2.deck-row-classification-refs.v1",
    ),
    "family_catalog": (
        "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
        "manafold.m2.5.b2.requirement-family-catalog.v1",
    ),
}
B1_FILES = {
    "citations": (
        "sources/m2_5/closures/B1/official_authority_citations.v3.json",
        "manafold.m2.5.b1.official-authority-citations.v3",
    ),
    "closure": (
        "sources/m2_5/closures/B1/official_authority_citation_closure.v2.json",
        "manafold.m2.5.b1.official-authority-citation-closure.v2",
    ),
}
IMMUTABLE_ROOTS = {
    "c": "sources/m2_5/closures/C",
    "b1": "sources/m2_5/closures/B1",
    "b2": "sources/m2_5/closures/B2",
    "authority": "sources/m2_5/authorities",
}

DECISION_SURFACE_MAP: dict[str, dict[str, Any]] = {
    "none intrinsic": {
        "family_id": "no_card_specific_decision",
        "status": "KNOWN_NOT_APPLICABLE",
        "actor": "none",
        "shape": "none",
    },
    "mana/payment selection may be strategically meaningful": {
        "family_id": "mana_payment_selection",
        "status": "KNOWN_REQUIRED",
        "actor": "controller_or_casting_player",
        "shape": "payment_plan_or_mana_selection",
    },
    (
        "ChooseOne eligible creature token to copy when populate resolves "
        "if more than one choice exists"
    ): {
        "family_id": "choose_one_token_copy_source",
        "status": "KNOWN_REQUIRED",
        "actor": "controller_of_resolving_spell",
        "shape": "choose_one_public_token",
    },
    "ChooseOne loyalty ability; target decisions; -8 derives X from life on resolution": {
        "family_id": "planeswalker_loyalty_ability_and_x",
        "status": "KNOWN_REQUIRED",
        "actor": "controller_of_planeswalker",
        "shape": "choose_one_loyalty_ability_plus_targets_or_x",
    },
    (
        "ChooseOne target creature card; later casting/payment decisions remain explicit; "
        "gained activated abilities generate their own decisions"
    ): {
        "family_id": "graveyard_target_and_dynamic_cast_chain",
        "status": "KNOWN_REQUIRED",
        "actor": "controller_of_ability_then_casting_player",
        "shape": "choose_one_public_graveyard_card_then_nested_actions",
    },
    (
        "ChooseMany exactly five untapped Zombies for cost plus ChooseOne target creature "
        "card in a graveyard"
    ): {
        "family_id": "choose_many_cost_payment_and_target",
        "status": "KNOWN_REQUIRED",
        "actor": "controller_of_activated_ability",
        "shape": "choose_many_exactly_five_then_choose_one",
    },
    (
        "ChooseOne loyalty ability; targets as required; each later graveyard cast remains "
        "its own explicit decision chain"
    ): {
        "family_id": "planeswalker_loyalty_and_graveyard_cast_chain",
        "status": "KNOWN_REQUIRED",
        "actor": "controller_of_planeswalker_then_casting_player",
        "shape": "choose_one_loyalty_ability_plus_targets_then_nested_actions",
    },
    (
        "activation/payment choices plus no hidden auto-selection; all eligible creature "
        "cards move under resolving controller"
    ): {
        "family_id": "activation_payment_and_mass_zone_move",
        "status": "KNOWN_REQUIRED",
        "actor": "controller_of_activated_ability",
        "shape": "activation_and_payment_then_forced_mass_move",
    },
    (
        "multiple staged ChooseMany decisions: active player first, then each other player "
        "in turn order with previous choices visible; mutate only after all selections; "
        "then continue resolution"
    ): {
        "family_id": "multi_actor_staged_choose_many",
        "status": "KNOWN_REQUIRED",
        "actor": "active_player_then_each_other_player_in_turn_order",
        "shape": "staged_choose_many_with_prior_selections_visible",
    },
    (
        "ChooseOne destroyed creature card from resulting graveyards if eligible; forced "
        "continuation resumes after mass destruction"
    ): {
        "family_id": "post_destruction_card_choice",
        "status": "KNOWN_REQUIRED",
        "actor": "controller_of_resolving_spell",
        "shape": "choose_one_from_public_resulting_graveyards",
    },
    (
        "ChooseOne existing creature type during resolution; domain must be complete, "
        "deterministic, and not stringly free-form player input"
    ): {
        "family_id": "choose_existing_creature_type",
        "status": "KNOWN_REQUIRED",
        "actor": "controller_of_resolving_spell",
        "shape": "choose_one_closed_creature_type_domain",
    },
}

INFO_SURFACE_MAP: dict[str, dict[str, Any]] = {
    "ordinary public/private projection only unless Oracle pin adds effects": {
        "family_id": "ordinary_public_private_projection",
        "status": "KNOWN_REQUIRED",
        "boundary": "public battlefield versus private hand/library",
    },
    "perspective-safe knowledge/visibility update": {
        "family_id": "perspective_knowledge_visibility_update",
        "status": "KNOWN_REQUIRED",
        "boundary": "perspective-visible knowledge and observed-event update",
    },
    "public permanent; ordinary hand/library privacy": {
        "family_id": "public_permanent_private_hand_library",
        "status": "KNOWN_REQUIRED",
        "boundary": "public permanent versus private hand/library",
    },
    "hidden-library/hand only through generic zone rules": {
        "family_id": "hidden_library_hand_zone_boundary",
        "status": "KNOWN_REQUIRED",
        "boundary": "hidden library/hand and generic zone transitions",
    },
    "chosen token is public; copied token receives new object/opaque identity": {
        "family_id": "public_token_copy_identity",
        "status": "KNOWN_REQUIRED",
        "boundary": "public token selection and new opaque identity",
    },
    "public loyalty/targets/effects": {
        "family_id": "public_loyalty_targets_effects",
        "status": "KNOWN_REQUIRED",
        "boundary": "public planeswalker loyalty and targets",
    },
    "public graveyard selection; granted-ability visibility derives from public card definition": {
        "family_id": "public_graveyard_dynamic_ability_visibility",
        "status": "KNOWN_REQUIRED",
        "boundary": "public graveyard and public granted ability set",
    },
    "public permanents/graveyard target": {
        "family_id": "public_permanent_and_graveyard_target",
        "status": "KNOWN_REQUIRED",
        "boundary": "public permanents and graveyard targets",
    },
    "loyalty/public targets public; library cards revealed only by normal mill zone movement": {
        "family_id": "public_loyalty_targets_and_mill_visibility",
        "status": "KNOWN_REQUIRED",
        "boundary": "public loyalty/targets with library-to-graveyard reveal consequence",
    },
    "all graveyards are public; new controller and modified characteristics are player-visible": {
        "family_id": "public_cross_graveyard_control_characteristics",
        "status": "KNOWN_REQUIRED",
        "boundary": "public graveyards, controller, and characteristics",
    },
    (
        "public controller/owner distinction; controller-sensitive player projection "
        "and observed events"
    ): {
        "family_id": "public_owner_controller_projection",
        "status": "KNOWN_REQUIRED",
        "boundary": "public owner/controller distinction and observed projection",
    },
    (
        "later decision-makers may know prior selections exactly as official ruling "
        "permits; no hidden premature mutation"
    ): {
        "family_id": "staged_selection_visibility",
        "status": "KNOWN_REQUIRED",
        "boundary": "prior public selections visible to later actors before mutation",
    },
    "selection from public graveyards; controller/characteristic changes visible": {
        "family_id": "public_graveyard_selection_controller_characteristics",
        "status": "KNOWN_REQUIRED",
        "boundary": "public graveyard selection with visible control/characteristic changes",
    },
    "chosen type public once made; board result public": {
        "family_id": "public_type_choice_board_effect",
        "status": "KNOWN_REQUIRED",
        "boundary": "public creature-type choice and battlefield result",
    },
}


class ScopeCensusValidationError(ValueError):
    """The selected-pair census is malformed or not source-bound."""


def _canonical(value: object) -> bytes:
    return (
        json.dumps(
            value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False
        )
        + "\n"
    ).encode("utf-8")


def _sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def _json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ScopeCensusValidationError(message)


def compute_artifact_content_sha256(artifact: dict[str, Any]) -> str:
    value = copy.deepcopy(artifact)
    value.pop("content_sha256", None)
    return _sha256(_canonical(value))


def _binding(path: str, schema: str | None, raw_sha256: str) -> dict[str, Any]:
    return {"path": path, "schema": schema, "raw_sha256": raw_sha256}


def load_census_artifacts(root: Path = ROOT) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for kind, filename in ARTIFACTS.items():
        result[kind] = _json(root / SCOPE_RELATIVE_PATH / filename)
    return result


def _validate_schema(artifact: dict[str, Any], root: Path) -> None:
    schema = _json(root / SCHEMA_RELATIVE_PATH)
    try:
        jsonschema.Draft202012Validator(schema).validate(artifact)
    except jsonschema.ValidationError as exc:
        raise ScopeCensusValidationError(
            f"{artifact.get('artifact_kind')} schema: {exc.message}"
        ) from exc


def _lock_context(root: Path) -> tuple[dict[str, Any], str, set[str], set[str]]:
    lock_path = root / LOCK_RELATIVE_PATH
    lock = load_lock(lock_path)
    validate_lock_document(lock)
    lock_sha = _sha256(lock_path.read_bytes())
    osis = {row["oracle_semantic_identity"] for deck in lock["decks"] for row in deck["cards"]}
    row_ids = {row["source_row_id"] for deck in lock["decks"] for row in deck["cards"]}
    return lock, lock_sha, osis, row_ids


def _repo_bindings(root: Path, paths: dict[str, tuple[str, str]]) -> list[dict[str, Any]]:
    values = []
    for path, schema in paths.values():
        values.append(_binding(path, schema, _sha256((root / path).read_bytes())))
    return sorted(values, key=lambda item: item["path"])


def _preserved_artifacts(root: Path) -> dict[str, list[dict[str, Any]]]:
    result: dict[str, list[dict[str, Any]]] = {}
    for category, relative in IMMUTABLE_ROOTS.items():
        result[category] = [
            _binding(path.relative_to(root).as_posix(), None, _sha256(path.read_bytes()))
            for path in sorted((root / relative).rglob("*"))
            if path.is_file()
        ]
    ranking = []
    for path in sorted((root / "sources/m2_5/pre_research/REV3").rglob("*")):
        if path.is_file() and "ranking" in path.name.lower():
            ranking.append(
                _binding(path.relative_to(root).as_posix(), None, _sha256(path.read_bytes()))
            )
    result["ranking"] = ranking
    return result


def _archive_context(archive_root: Path, lock: dict[str, Any]) -> dict[str, Any]:
    archive_path = (
        archive_root / "m2_5/Manafold_M2_5_Pre_Research_ALL_ARTIFACTS_REV3.zip"
    ).resolve()
    raw_archive = archive_path.read_bytes()
    _require(_sha256(raw_archive) == EXPECTED_SOURCE_PACKAGE_SHA256, "REV3 package digest mismatch")
    with zipfile.ZipFile(io.BytesIO(raw_archive)) as archive:
        manifest_raw = archive.read("Manafold_M2_5_Package_Manifest_REV3.json")
        manifest = json.loads(manifest_raw.decode("utf-8"))
        entries = {entry["path"]: entry for entry in manifest["entries"]}

        def member(path: str) -> tuple[bytes, dict[str, Any]]:
            raw = archive.read(path)
            expected = entries[path]
            _require(len(raw) == expected["bytes"], f"archive member size mismatch: {path}")
            _require(_sha256(raw) == expected["sha256"], f"archive member digest mismatch: {path}")
            return raw, _binding(path, None, expected["sha256"])

        resolution_raw, resolution_binding = member("inputs/deck_row_source_resolution_REV3.csv")
        oracle_raw, oracle_binding = member("source/raw/oracle_cards_selected_REV3.jsonl")
        index_raw, index_binding = member("source/raw/source_record_index_REV3.csv")
        surface_raw, surface_binding = member("inputs/card_semantic_classification_REV3.json")
        oracle_records: dict[str, dict[str, Any]] = {}
        oracle_raw_digests: dict[str, str] = {}
        for line in oracle_raw.splitlines(keepends=True):
            record = json.loads(line)
            oracle_records[record["id"]] = record
            oracle_raw_digests[record["id"]] = _sha256(line)
        surface_records = {
            record["oracle_semantic_identity"]: record
            for record in json.loads(surface_raw.decode("utf-8"))
        }
        resolution_rows = list(
            csv.DictReader(io.StringIO(resolution_raw.decode("utf-8"), newline=""))
        )
        index_rows = list(csv.DictReader(io.StringIO(index_raw.decode("utf-8"), newline="")))
        oracle_index = {
            row["source_record_id"]: row for row in index_rows if row["bulk_type"] == "oracle_cards"
        }
        return {
            "entries": entries,
            "resolution_rows": resolution_rows,
            "oracle_records": oracle_records,
            "oracle_raw_digests": oracle_raw_digests,
            "oracle_index": oracle_index,
            "surface_records": surface_records,
            "bindings": [resolution_binding, oracle_binding, index_binding, surface_binding],
        }


def _source_ref(lock_rows: list[dict[str, Any]]) -> dict[str, Any]:
    return {
        "deck_names": sorted({row["deck_name"] for row in lock_rows}),
        "deck_row_ids": sorted(row["source_row_id"] for row in lock_rows),
        "card_names": sorted({row["card_name"] for row in lock_rows}),
        "oracle_semantic_identity": lock_rows[0]["oracle_semantic_identity"],
        "oracle_source_record_id": lock_rows[0]["oracle_source_record_id"],
        "source_record_raw_sha256": lock_rows[0]["oracle_source_record_raw_sha256"],
        "normalized_record_sha256": lock_rows[0]["oracle_normalized_record_sha256"],
    }


def _family_owner_roles(family: dict[str, Any]) -> list[str]:
    metrics = family["historical_rev3"]["record"]["metric_memberships"]
    roles = []
    if metrics.get("decision"):
        roles.append("M3_DECISION_PROTOCOL")
    if metrics.get("information_identity"):
        roles.append("M3_INFORMATION_SAFETY")
    if metrics.get("interaction"):
        roles.append("M3_RULES_INTERACTION")
    if not roles:
        roles.append("M3_RULES_CAPABILITY")
    return sorted(roles)


def _common(
    root: Path,
    lock: dict[str, Any],
    lock_sha: str,
    archive: dict[str, Any],
    kind: str,
) -> dict[str, Any]:
    return {
        "schema": SCHEMA_ID,
        "artifact_kind": kind,
        "task_id": TASK_ID,
        "status": "PASS",
        "selected_pair": {
            "lock_path": LOCK_RELATIVE_PATH.as_posix(),
            "lock_sha256": lock_sha,
            "deck_ids": [deck["deck_id"] for deck in lock["decks"]],
            "deck_names": [deck["deck_name"] for deck in lock["decks"]],
        },
        "source_package_sha256": EXPECTED_SOURCE_PACKAGE_SHA256,
        "evidence_bindings": {
            "b1": _repo_bindings(root, B1_FILES),
            "b2": _repo_bindings(root, B2_FILES),
            "rev3": sorted(archive["bindings"], key=lambda item: item["path"]),
        },
        "preserved_artifact_digests": _preserved_artifacts(root),
        "selected_decks": [
            {"deck_id": deck["deck_id"], "deck_name": deck["deck_name"], "player": deck["player"]}
            for deck in lock["decks"]
        ],
        "record_counts": {},
        "records": [],
        "families": [],
        "dependency_edges": [],
        "direct_roots": [],
        "resolved_families": [],
        "unresolved_scope_obligations": [],
        "high_risk_outliers": [],
    }


def _build_context(root: Path, archive_root: Path) -> dict[str, Any]:
    lock, lock_sha, osis, row_ids = _lock_context(root)
    verify_pinned_archive(lock, archive_root)
    archive = _archive_context(archive_root, lock)
    b2_classifications = _json(root / B2_FILES["classifications"][0])["classifications"]
    with (root / B2_FILES["projection"][0]).open(encoding="utf-8", newline="") as handle:
        projection = list(csv.DictReader(handle))
    catalog = {
        family["family_id"]: family
        for family in _json(root / B2_FILES["family_catalog"][0])["families"]
    }
    classifications = {record["oracle_semantic_identity"]: record for record in b2_classifications}
    selected_projection = [
        row for row in projection if row["deck_id"] in {"Token Triumph", "Grave Danger"}
    ]
    lock_rows = []
    for deck in lock["decks"]:
        for row in deck["cards"]:
            enriched = dict(row)
            enriched["deck_id"] = deck["deck_id"]
            enriched["deck_name"] = deck["deck_name"]
            enriched["player"] = deck["player"]
            lock_rows.append(enriched)
    rows_by_osi: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in lock_rows:
        rows_by_osi[row["oracle_semantic_identity"]].append(row)
    _require({row["source_row_id"] for row in lock_rows} == row_ids, "lock/source row set mismatch")
    _require(
        {row["oracle_semantic_identity"] for row in lock_rows} == osis,
        "lock/Oracle identity set mismatch",
    )
    return {
        "root": root,
        "lock": lock,
        "lock_sha": lock_sha,
        "archive": archive,
        "catalog": catalog,
        "classifications": classifications,
        "projection": selected_projection,
        "rows_by_osi": rows_by_osi,
        "osis": sorted(osis),
    }


def _build_capability(context: dict[str, Any]) -> dict[str, Any]:
    artifact = _common(
        context["root"], context["lock"], context["lock_sha"], context["archive"], "capability"
    )
    families: dict[str, dict[str, Any]] = {}
    records = []
    edges = 0
    for osi in context["osis"]:
        classification = context["classifications"].get(osi)
        _require(classification is not None, f"missing B2 classification for Oracle identity {osi}")
        rows = context["rows_by_osi"][osi]
        assignments = []
        for assignment in classification["requirement_assignments"]:
            family_id = assignment["requirement_family_id"]
            family = context["catalog"].get(family_id)
            _require(family is not None, f"unknown B2 family {family_id}")
            _require(
                family["status"] == "ACTIVE" and family["terminal_assignable"],
                f"nonterminal B2 family {family_id}",
            )
            family_record = families.setdefault(
                family_id,
                {
                    "family_id": family_id,
                    "canonical_name": family["canonical_name"],
                    "status": family["status"],
                    "terminal_assignable": family["terminal_assignable"],
                    "precise_semantic_definition_sha256": _sha256(
                        family["precise_semantic_definition"].encode("utf-8")
                    ),
                    "semantic_owner_roles": _family_owner_roles(family),
                    "semantic_owner_status": "SCOPE_ROLE_REQUIRED",
                    "edge_count": 0,
                    "oracle_identity_count": 0,
                    "deck_row_count": 0,
                },
            )
            family_record["edge_count"] += len(rows)
            family_record["oracle_identity_count"] += 1
            family_record["deck_row_count"] += len(rows)
            edges += len(rows)
            assignments.append(
                {
                    "family_id": family_id,
                    "classification_identity_sha256": classification["classification_identity"][
                        "digest_hex"
                    ],
                    "review_status": classification["review_status"],
                    "evidence_basis": assignment["evidence_basis"],
                    "assignment_evidence_sha256": _sha256(_canonical(assignment)),
                    "source_evidence_digest": classification["source_evidence_digest"][
                        "digest_hex"
                    ],
                    "family_boundary_sha256": family_record["precise_semantic_definition_sha256"],
                }
            )
        records.append(
            {
                **_source_ref(rows),
                "classification_identity_sha256": classification["classification_identity"][
                    "digest_hex"
                ],
                "review_status": classification["review_status"],
                "capability_assignments": sorted(assignments, key=lambda item: item["family_id"]),
            }
        )
    artifact["records"] = sorted(records, key=lambda item: item["oracle_semantic_identity"])
    artifact["families"] = sorted(families.values(), key=lambda item: item["family_id"])
    artifact["high_risk_outliers"] = _high_risk(context)
    artifact["record_counts"] = {
        "selected_deck_rows": len(context["projection"]),
        "selected_oracle_identities": len(records),
        "capability_families": len(families),
        "capability_assignment_edges": edges,
        "unique_osi_capability_edges": sum(
            len(record["capability_assignments"]) for record in records
        ),
        "per_card_records": len(records),
    }
    return artifact


def _high_risk(context: dict[str, Any]) -> list[dict[str, Any]]:
    results = []
    for osi in context["osis"]:
        surface = context["archive"]["surface_records"].get(osi)
        if surface and surface.get("classification_tier") == "HIGH-RISK":
            results.append(
                {
                    "oracle_semantic_identity": osi,
                    "card_names": sorted({row["card_name"] for row in context["rows_by_osi"][osi]}),
                    "risk_tags": surface.get("risk_tags", ""),
                    "classification_tier": surface.get("classification_tier"),
                    "evidence_member": "inputs/card_semantic_classification_REV3.json",
                }
            )
    return sorted(results, key=lambda item: item["oracle_semantic_identity"])


def _surface_evidence(context: dict[str, Any], osi: str) -> dict[str, Any]:
    record = context["classifications"][osi]
    return {
        "oracle_semantic_identity": osi,
        "b2_classification_identity_sha256": record["classification_identity"]["digest_hex"],
        "b2_evidence_path": B2_FILES["classifications"][0],
        "rev3_surface_member": "inputs/card_semantic_classification_REV3.json",
        "rev3_surface_locator": ["oracle_semantic_identity", osi],
        "source_record_raw_sha256": record["source_identity"]["source_record_raw_sha256"],
    }


def _build_surface_artifact(
    context: dict[str, Any], kind: str, mapping: dict[str, dict[str, Any]], field: str
) -> dict[str, Any]:
    artifact = _common(
        context["root"], context["lock"], context["lock_sha"], context["archive"], kind
    )
    surfaces: dict[str, dict[str, Any]] = {}
    records = []
    unresolved = []
    for osi in context["osis"]:
        source = context["archive"]["surface_records"].get(osi)
        _require(source is not None, f"missing source surface record {osi}")
        surface = source[field]
        definition = mapping.get(surface)
        if definition is None:
            if surface == "BLOCKED_CURRENT_ORACLE_PER_CARD_EXTRACTION":
                definition = {
                    "family_id": f"{kind}_surface_unresolved_current_oracle",
                    "status": "UNRESOLVED_SCOPE_OBLIGATION",
                    "actor": "UNRESOLVED",
                    "shape": "UNRESOLVED",
                    "boundary": "UNRESOLVED_CURRENT_ORACLE_PER_CARD_EXTRACTION",
                }
            elif field == "decision_surface":
                definition = {
                    "family_id": "card_specific_decision_surface_unresolved",
                    "status": "UNRESOLVED_SCOPE_OBLIGATION",
                    "actor": "UNRESOLVED",
                    "shape": "UNRESOLVED",
                }
            else:
                definition = {
                    "family_id": "information_surface_unresolved",
                    "status": "UNRESOLVED_SCOPE_OBLIGATION",
                    "boundary": "UNRESOLVED",
                }
        surfaces.setdefault(
            definition["family_id"],
            {
                "family_id": definition["family_id"],
                "source_surface": surface,
                **definition,
                "owning_capability_families": [],
                "oracle_identity_count": 0,
            },
        )
        family = surfaces[definition["family_id"]]
        family["oracle_identity_count"] += 1
        rows = context["rows_by_osi"][osi]
        classification = context["classifications"][osi]
        family["owning_capability_families"] = sorted(
            set(family["owning_capability_families"])
            | {a["requirement_family_id"] for a in classification["requirement_assignments"]}
        )
        records.append(
            {
                **_source_ref(rows),
                "surface": surface,
                **definition,
                "owning_capability_families": sorted(
                    assignment["requirement_family_id"]
                    for assignment in context["classifications"][osi]["requirement_assignments"]
                ),
                "evidence": _surface_evidence(context, osi),
            }
        )
        if definition["status"] == "UNRESOLVED_SCOPE_OBLIGATION":
            unresolved.append(
                {
                    "obligation_id": f"{kind}:{osi}",
                    "reason_code": "SOURCE_SURFACE_UNRESOLVED",
                    "subject": osi,
                    "evidence_refs": [_surface_evidence(context, osi)],
                }
            )
    artifact["records"] = sorted(records, key=lambda item: item["oracle_semantic_identity"])
    artifact["families"] = sorted(surfaces.values(), key=lambda item: item["family_id"])
    artifact["unresolved_scope_obligations"] = unresolved
    artifact["high_risk_outliers"] = _high_risk(context)
    artifact["record_counts"] = {
        "selected_oracle_identities": len(records),
        f"{kind}_families": len(surfaces),
        f"{kind}_obligations": len(records),
    }
    return artifact


def _build_generated_objects(context: dict[str, Any]) -> dict[str, Any]:
    artifact = _common(
        context["root"],
        context["lock"],
        context["lock_sha"],
        context["archive"],
        "generated_object",
    )
    tokens: dict[str, dict[str, Any]] = {}
    references: dict[str, dict[str, Any]] = {}
    for osi in context["osis"]:
        record = context["archive"]["oracle_records"][
            context["rows_by_osi"][osi][0]["oracle_source_record_id"]
        ]
        for part in record.get("all_parts") or []:
            key = (
                f"token/{part.get('name')}|{part.get('type_line')}"
                if part.get("component") == "token"
                else f"reference/{part['id']}"
            )
            target = tokens if part.get("component") == "token" else references
            target.setdefault(
                key,
                {
                    "object_class_id": key,
                    "object_kind": "TOKEN_CLASS"
                    if part.get("component") == "token"
                    else "REFERENCED_OBJECT",
                    "name": part.get("name"),
                    "type_line": part.get("type_line"),
                    "source_identity_ids": [],
                    "source_part_bindings": [],
                    "source_card_osis": [],
                    "status": "KNOWN_REQUIRED",
                },
            )
            target[key]["source_card_osis"].append(osi)
            target[key]["source_identity_ids"].append(part["id"])
            target[key]["source_part_bindings"].append(
                {
                    "oracle_source_record_id": record["id"],
                    "source_record_raw_sha256": context["archive"]["oracle_raw_digests"][
                        record["id"]
                    ],
                    "all_part_id": part["id"],
                    "component": part.get("component"),
                    "name": part.get("name"),
                    "type_line": part.get("type_line"),
                }
            )
    semantic = [
        {
            "object_class_id": "copied_token_instance",
            "object_kind": "COPY_INSTANCE",
            "status": "KNOWN_REQUIRED",
            "source_card_osis": ["04046ae4-5c51-434b-930c-f3b1d348bf4b"],
            "owning_capability_families": ["cap.copiable_token_values", "cap.populate_token_copy"],
        },
        {
            "object_class_id": "new_zone_incarnation",
            "object_kind": "ZONE_CHANGE_IDENTITY",
            "status": "KNOWN_REQUIRED",
            "source_card_osis": sorted(context["osis"]),
            "owning_capability_families": [
                "cap.graveyard",
                "cap.reanimation",
                "cap.reanimation_under_your_control",
            ],
        },
        {
            "object_class_id": "planeswalker_loyalty_and_counter_state",
            "object_kind": "COUNTER_STATE",
            "status": "KNOWN_REQUIRED",
            "source_card_osis": [
                "1d5ff280-7f4d-4801-aefd-9b6e6b1f2818",
                "4f66489c-5a19-40ad-9126-461fa8231f1b",
            ],
            "owning_capability_families": [
                "cap.loyalty_activation_rules",
                "cap.loyalty_counters",
                "cap.counters",
            ],
        },
        {
            "object_class_id": "owner_controller_distinct_object",
            "object_kind": "CONTROL_OWNER_STATE",
            "status": "KNOWN_REQUIRED",
            "source_card_osis": sorted(context["osis"]),
            "owning_capability_families": [
                "cap.owner_controller_separation",
                "cap.controller_assignment_on_zone_entry",
            ],
        },
        {
            "object_class_id": "emblem",
            "object_kind": "EMBLEM",
            "status": "KNOWN_NOT_APPLICABLE",
            "source_card_osis": [],
            "owning_capability_families": [],
            "evidence_refs": [
                {"reason": "no selected Oracle all_parts token/emblem record or B2 assignment"}
            ],
        },
    ]
    artifact["records"] = sorted(
        [*tokens.values(), *references.values(), *semantic],
        key=lambda item: item["object_class_id"],
    )
    artifact["record_counts"] = {
        "generated_object_classes": len(tokens) + len(semantic),
        "token_classes": len(tokens),
        "referenced_object_records": len(references),
        "semantic_object_obligations": len(semantic),
    }
    artifact["high_risk_outliers"] = _high_risk(context)
    return artifact


def _build_closure(context: dict[str, Any], capability: dict[str, Any]) -> dict[str, Any]:
    artifact = _common(
        context["root"],
        context["lock"],
        context["lock_sha"],
        context["archive"],
        "recursive_capability_closure",
    )
    roots = sorted(family["family_id"] for family in capability["families"])
    artifact["direct_roots"] = roots
    artifact["resolved_families"] = roots
    artifact["dependency_edges"] = []
    artifact["families"] = capability["families"]
    artifact["record_counts"] = {
        "direct_capability_roots": len(roots),
        "resolved_recursive_capability_families": len(roots),
        "explicit_dependency_edges": 0,
    }
    artifact["unresolved_scope_obligations"] = [
        {
            "obligation_id": f"recursive-dependency:{family_id}",
            "reason_code": "NO_ACCEPTED_TRANSITIVE_B2_DEPENDENCY_EDGE",
            "subject": family_id,
            "evidence_refs": [{"b2_family_id": family_id}],
        }
        for family_id in roots
    ]
    artifact["high_risk_outliers"] = capability["high_risk_outliers"]
    return artifact


def build_census_artifacts(
    root: Path = ROOT, archive_root: Path | None = None
) -> dict[str, dict[str, Any]]:
    configured = archive_root or Path(os.environ[ARCHIVE_ENV_VAR])
    context = _build_context(root, configured)
    capability = _build_capability(context)
    validate_b2_projection_rows(context["projection"], context["classifications"], capability)
    decision = _build_surface_artifact(
        context, "decision", DECISION_SURFACE_MAP, "decision_surface"
    )
    information = _build_surface_artifact(
        context, "information", INFO_SURFACE_MAP, "information_surface"
    )
    generated = _build_generated_objects(context)
    closure = _build_closure(context, capability)
    artifacts = {
        "capability": capability,
        "decision": decision,
        "information": information,
        "generated_object": generated,
        "recursive_capability_closure": closure,
    }
    for artifact in artifacts.values():
        artifact["content_sha256"] = compute_artifact_content_sha256(artifact)
    return artifacts


def validate_b2_projection_rows(
    selected_projection: list[dict[str, str]],
    classifications: dict[str, dict[str, Any]],
    capability: dict[str, Any],
) -> None:
    """Join every selected projection row to its B2 classification and census record."""

    record_by_row = {
        row_id: record for record in capability["records"] for row_id in record["deck_row_ids"]
    }
    seen_rows: set[str] = set()
    calculated_edges = 0
    for row in selected_projection:
        row_id = row["deck_row_id"]
        _require(row_id not in seen_rows, f"B2 projection duplicate row: {row_id}")
        seen_rows.add(row_id)
        census_record = record_by_row.get(row_id)
        _require(census_record is not None, f"B2 projection row is absent from census: {row_id}")
        osi = row["oracle_semantic_identity"]
        _require(
            census_record["oracle_semantic_identity"] == osi,
            f"B2 projection OSI mismatch: {row_id}",
        )
        classification = classifications.get(osi)
        _require(classification is not None, f"B2 projection OSI lacks classification: {osi}")
        _require(
            row["terminal_classification_identity"]
            == classification["classification_identity"]["digest_hex"],
            f"B2 projection classification identity mismatch: {row_id}",
        )
        _require(
            row["classification_status"] == classification["review_status"],
            f"B2 projection classification status mismatch: {row_id}",
        )
        projection_requirements = json.loads(row["terminal_requirement_ids"])
        classification_requirements = [
            assignment["requirement_family_id"]
            for assignment in classification["requirement_assignments"]
        ]
        _require(
            sorted(projection_requirements) == sorted(classification_requirements),
            f"B2 projection requirement IDs mismatch: {row_id}",
        )
        census_requirements = [
            assignment["family_id"] for assignment in census_record["capability_assignments"]
        ]
        _require(
            sorted(census_requirements) == sorted(projection_requirements),
            f"B2 projection/census assignment mismatch: {row_id}",
        )
        calculated_edges += len(projection_requirements)
    _require(seen_rows == set(record_by_row), "B2 projection/census row set mismatch")
    _require(calculated_edges == 628, "B2 projection edge count mismatch")


def validate_generated_object_records(
    artifact: dict[str, Any], root: Path, archive_root: Path
) -> None:
    """Verify generated/reference records against exact Oracle all_parts rows."""

    lock, _, osis, _ = _lock_context(root)
    archive = _archive_context(archive_root, lock)
    source_by_osi = {
        row["oracle_semantic_identity"]: row for deck in lock["decks"] for row in deck["cards"]
    }
    for record in artifact["records"]:
        source_osis = set(record.get("source_card_osis", []))
        _require(source_osis.issubset(osis), "generated object has unbound source identity")
        if record["object_kind"] not in {"TOKEN_CLASS", "REFERENCED_OBJECT"}:
            continue
        bindings = record.get("source_part_bindings", [])
        _require(bindings, "generated object all_parts bindings are missing")
        expected_pairs: set[tuple[str, str]] = set()
        for source_osi in source_osis:
            source_record_id = source_by_osi[source_osi]["oracle_source_record_id"]
            source_record = archive["oracle_records"].get(source_record_id)
            _require(source_record is not None, "generated object source card record is missing")
            for part in source_record.get("all_parts") or []:
                if (
                    part.get("component")
                    == ("token" if record["object_kind"] == "TOKEN_CLASS" else "combo_piece")
                    and part.get("name") == record.get("name")
                    and part.get("type_line") == record.get("type_line")
                ):
                    expected_pairs.add((source_record_id, part["id"]))
        actual_pairs = {
            (binding["oracle_source_record_id"], binding["all_part_id"]) for binding in bindings
        }
        _require(
            actual_pairs == expected_pairs,
            "generated object all_parts binding set is incomplete or foreign",
        )
        _require(
            {binding["all_part_id"] for binding in bindings}
            == set(record.get("source_identity_ids", [])),
            "generated object all_parts identity set mismatch",
        )
        for binding in bindings:
            source_record_id = binding["oracle_source_record_id"]
            source_record = archive["oracle_records"].get(source_record_id)
            _require(
                source_record is not None, "generated object all_parts source record is missing"
            )
            _require(
                archive["oracle_raw_digests"][source_record_id]
                == binding["source_record_raw_sha256"],
                "generated object all_parts raw digest mismatch",
            )
            matching_parts = [
                part
                for part in source_record.get("all_parts") or []
                if part.get("id") == binding["all_part_id"]
                and part.get("component") == binding["component"]
                and part.get("name") == binding["name"]
                and part.get("type_line") == binding["type_line"]
            ]
            _require(matching_parts, "generated object all_parts source binding mismatch")
        if record.get("source_part_bindings"):
            _require(
                all(
                    binding["name"] == record.get("name")
                    for binding in record["source_part_bindings"]
                ),
                "generated object all_parts name mismatch",
            )
            _require(
                all(
                    binding["type_line"] == record.get("type_line")
                    for binding in record["source_part_bindings"]
                ),
                "generated object all_parts type mismatch",
            )
            _require(
                set(record.get("source_identity_ids", []))
                == {binding["all_part_id"] for binding in record["source_part_bindings"]},
                "generated object all_parts identity set mismatch",
            )


def validate_census_set(
    artifacts: dict[str, dict[str, Any]],
    root: Path = ROOT,
    archive_root: Path | None = None,
) -> None:
    _require(
        set(artifacts) == set(ARTIFACTS),
        "census artifact set is incomplete or has an extra artifact",
    )
    for artifact in artifacts.values():
        _require(
            len(artifact.get("selected_decks", [])) == 2
            and {deck.get("deck_name") for deck in artifact["selected_decks"]}
            == {"Token Triumph", "Grave Danger"},
            "selected pair is not exact",
        )
        _validate_schema(artifact, root)
        _require(
            artifact["source_package_sha256"] == EXPECTED_SOURCE_PACKAGE_SHA256,
            "source package binding mismatch",
        )
        _require(
            artifact["content_sha256"] == compute_artifact_content_sha256(artifact),
            f"{artifact['artifact_kind']} content digest mismatch",
        )
        _require(
            all(deck["player"] in {1, 2} for deck in artifact["selected_decks"]),
            "selected pair has an invalid player",
        )
        for _category, bindings in artifact["preserved_artifact_digests"].items():
            for binding in bindings:
                path = root / binding["path"]
                _require(path.is_file(), f"preserved artifact missing: {binding['path']}")
                _require(
                    _sha256(path.read_bytes()) == binding["raw_sha256"],
                    f"preserved artifact changed: {binding['path']}",
                )
    capability = artifacts["capability"]
    records = capability["records"]
    lock, _, osis, row_ids = _lock_context(root)
    expected_b1 = _repo_bindings(root, B1_FILES)
    expected_b2 = _repo_bindings(root, B2_FILES)
    for artifact in artifacts.values():
        _require(artifact["evidence_bindings"]["b1"] == expected_b1, "B1 evidence binding drift")
        _require(artifact["evidence_bindings"]["b2"] == expected_b2, "B2 evidence binding drift")
    classifications = {
        record["oracle_semantic_identity"]: record
        for record in _json(root / B2_FILES["classifications"][0])["classifications"]
    }
    with (root / B2_FILES["projection"][0]).open(encoding="utf-8", newline="") as handle:
        selected_projection = [
            row
            for row in csv.DictReader(handle)
            if row["deck_id"] in {"Token Triumph", "Grave Danger"}
        ]
    validate_b2_projection_rows(selected_projection, classifications, capability)
    _require(
        {record["oracle_semantic_identity"] for record in records} == osis,
        "capability census Oracle identity set mismatch",
    )
    seen_rows: set[str] = set()
    seen_edges: set[tuple[str, str]] = set()
    family_ids = {family["family_id"] for family in capability["families"]}
    catalog = {
        family["family_id"] for family in _json(root / B2_FILES["family_catalog"][0])["families"]
    }
    for record in records:
        for row_id in record["deck_row_ids"]:
            _require(
                row_id in row_ids and row_id not in seen_rows,
                f"duplicate or unbound capability row: {row_id}",
            )
            seen_rows.add(row_id)
        for assignment in record["capability_assignments"]:
            family_id = assignment["family_id"]
            _require(
                family_id in catalog and family_id in family_ids,
                f"unbound B2 capability family: {family_id}",
            )
            source_classification = classifications[record["oracle_semantic_identity"]]
            _require(
                assignment["classification_identity_sha256"]
                == source_classification["classification_identity"]["digest_hex"],
                f"classification identity mismatch for {record['oracle_semantic_identity']}",
            )
            _require(
                family_id
                in {
                    item["requirement_family_id"]
                    for item in source_classification["requirement_assignments"]
                },
                f"capability assignment is not present in accepted B2 evidence: {family_id}",
            )
            key = (record["oracle_semantic_identity"], family_id)
            _require(key not in seen_edges, f"duplicate capability assignment edge: {key}")
            seen_edges.add(key)
    _require(seen_rows == row_ids, "capability census does not cover every selected deck row")
    _require(
        capability["record_counts"]["selected_deck_rows"] == 144, "selected deck row count mismatch"
    )
    _require(
        capability["record_counts"]["selected_oracle_identities"] == 140,
        "selected Oracle identity count mismatch",
    )
    _require(
        capability["record_counts"]["capability_families"] == 125,
        "selected capability family count mismatch",
    )
    _require(
        capability["record_counts"]["capability_assignment_edges"] == 628,
        "selected capability edge count mismatch",
    )
    for kind in ("decision", "information"):
        seen_surface_osis: set[str] = set()
        for record in artifacts[kind]["records"]:
            osi = record["oracle_semantic_identity"]
            _require(osi in osis, f"{kind} census has unbound Oracle identity: {osi}")
            _require(osi not in seen_surface_osis, f"duplicate {kind} census identity: {osi}")
            seen_surface_osis.add(osi)
            _require(
                set(record["deck_row_ids"]).issubset(row_ids),
                f"{kind} census has unbound source row",
            )
            _require(
                record["evidence"]["oracle_semantic_identity"] == osi,
                f"{kind} evidence identity mismatch",
            )
            _require(
                set(record["owning_capability_families"]).issubset(family_ids),
                f"{kind} census has an unbound owning capability",
            )
        _require(
            seen_surface_osis == osis,
            f"{kind} census does not cover all selected Oracle identities",
        )
    generated_seen: set[str] = set()
    for record in artifacts["generated_object"]["records"]:
        class_id = record["object_class_id"]
        _require(class_id not in generated_seen, f"duplicate generated-object class: {class_id}")
        generated_seen.add(class_id)
        _require(
            set(record.get("source_card_osis", [])).issubset(osis),
            "generated object has unbound source identity",
        )
    _require(
        len(generated_seen)
        == artifacts["generated_object"]["record_counts"]["generated_object_classes"]
        + artifacts["generated_object"]["record_counts"]["referenced_object_records"],
        "generated-object record count mismatch",
    )
    closure = artifacts["recursive_capability_closure"]
    _require(
        set(closure["direct_roots"]) == family_ids,
        "recursive closure roots differ from capability families",
    )
    _require(
        set(closure["resolved_families"]) >= family_ids,
        "recursive closure omits direct capability roots",
    )
    _require(
        not any(artifact["artifact_kind"] == "authority" for artifact in artifacts.values()),
        "Authority artifact admitted",
    )
    if archive_root is not None:
        archive = _archive_context(archive_root, lock)
        expected_rev3 = sorted(archive["bindings"], key=lambda item: item["path"])
        verify_pinned_archive(lock, archive_root)
        source_by_osi: dict[str, dict[str, Any]] = {}
        for deck in lock["decks"]:
            for row in deck["cards"]:
                source_by_osi.setdefault(row["oracle_semantic_identity"], row)
        for artifact in artifacts.values():
            _require(
                artifact["evidence_bindings"]["rev3"] == expected_rev3,
                "REV3 evidence binding drift",
            )
            for record in artifact["records"]:
                osi = record.get("oracle_semantic_identity")
                if not isinstance(osi, str) or osi not in source_by_osi:
                    continue
                expected = source_by_osi[osi]
                _require(
                    record.get("oracle_source_record_id") == expected["oracle_source_record_id"],
                    f"source record binding mismatch for {osi}",
                )
                _require(
                    record.get("source_record_raw_sha256")
                    == expected["oracle_source_record_raw_sha256"],
                    f"source raw digest mismatch for {osi}",
                )
                _require(
                    record.get("normalized_record_sha256")
                    == expected["oracle_normalized_record_sha256"],
                    f"source normalized digest mismatch for {osi}",
                )
        validate_generated_object_records(artifacts["generated_object"], root, archive_root)


def _write_artifacts(artifacts: dict[str, dict[str, Any]], root: Path) -> None:
    target = root / SCOPE_RELATIVE_PATH
    target.mkdir(parents=True, exist_ok=True)
    for kind, filename in ARTIFACTS.items():
        (target / filename).write_text(
            json.dumps(artifacts[kind], ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--archive-root", type=Path)
    args = parser.parse_args()
    try:
        archive_root = args.archive_root or Path(os.environ[ARCHIVE_ENV_VAR])
        if args.write:
            artifacts = build_census_artifacts(ROOT, archive_root)
            _write_artifacts(artifacts, ROOT)
        artifacts = load_census_artifacts(ROOT)
        validate_census_set(artifacts, ROOT, archive_root)
    except (OSError, KeyError, ValueError, json.JSONDecodeError, zipfile.BadZipFile) as exc:
        print(f"BLOCKED: {exc}")
        return 2
    print("PASS: selected-pair CDI census artifacts and immutable evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
