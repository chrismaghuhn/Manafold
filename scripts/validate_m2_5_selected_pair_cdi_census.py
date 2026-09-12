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
CENSUS_SCHEMA_RELATIVE_PATH = Path("schemas/m2-5-selected-pair-cdi-census.v1.schema.json")
CLOSURE_SCHEMA_RELATIVE_PATH = Path(
    "schemas/m2-5-selected-pair-recursive-capability-closure.v2.schema.json"
)
SCOPE_RELATIVE_PATH = Path("sources/m2_5/scope")
TASK_ID = "M2_5_SELECTED_PAIR_CDI_CENSUS_01"
CENSUS_SCHEMA_ID = "manafold.m2.5.selected-pair-cdi-census.v1"
CLOSURE_SCHEMA_ID = "manafold.m2.5.selected-pair-recursive-capability-closure.v2"
ARCHIVE_ENV_VAR = "MANAFOLD_SOURCE_ARCHIVE"
HISTORICAL_CLOSURE_RELATIVE_PATH = (
    "sources/m2_5/scope/selected_pair_recursive_capability_closure.v1.json"
)

DEPENDENCY_KINDS = frozenset(
    {
        "SEMANTIC_PREREQUISITE",
        "STATE_MODEL_PREREQUISITE",
        "DECISION_PREREQUISITE",
        "INFORMATION_PREREQUISITE",
        "FORMAT_PREREQUISITE",
        "GENERATED_OBJECT_PREREQUISITE",
    }
)

ARTIFACTS = {
    "capability": "selected_pair_capability_census.v1.json",
    "decision": "selected_pair_decision_census.v1.json",
    "information": "selected_pair_information_census.v1.json",
    "generated_object": "selected_pair_generated_object_census.v1.json",
    "recursive_capability_closure": "selected_pair_recursive_capability_closure.v2.json",
}

SEMANTIC_GENERATED_OBJECT_SPECS = {
    "copied_token_instance": {
        "object_kind": "COPY_INSTANCE",
        "status": "KNOWN_REQUIRED",
        "binding_mode": "B2_ASSIGNMENT",
        "source_selector_family_ids": ("cap.populate_token_copy",),
        "evidence_family_ids": (
            "cap.copiable_token_values",
            "cap.populate_token_copy",
        ),
    },
    "new_zone_incarnation": {
        "object_kind": "ZONE_CHANGE_IDENTITY",
        "status": "KNOWN_REQUIRED",
        "binding_mode": "SCOPE_INVARIANT",
        "scope_contracts": (
            ("docs/DOMAIN_MODEL.md", "Identity families and Zone model"),
            ("docs/cards/CAPABILITY_MODEL.md", "Closure sources"),
        ),
    },
    "planeswalker_loyalty_and_counter_state": {
        "object_kind": "COUNTER_STATE",
        "status": "KNOWN_REQUIRED",
        "binding_mode": "B2_ASSIGNMENT",
        "source_selector_family_ids": ("cap.planeswalker",),
        "evidence_family_ids": (
            "cap.counters",
            "cap.loyalty_activation_rules",
            "cap.loyalty_counters",
            "cap.planeswalker",
        ),
    },
    "owner_controller_distinct_object": {
        "object_kind": "CONTROL_OWNER_STATE",
        "status": "KNOWN_REQUIRED",
        "binding_mode": "B2_ASSIGNMENT",
        "source_selector_family_ids": (
            "cap.controller_assignment_on_zone_entry",
            "cap.owner_controller_separation",
        ),
        "evidence_family_ids": (
            "cap.controller_assignment_on_zone_entry",
            "cap.owner_controller_separation",
        ),
    },
    "emblem": {
        "object_kind": "EMBLEM",
        "status": "KNOWN_NOT_APPLICABLE",
        "binding_mode": "REV3_NEGATIVE_SCAN",
    },
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
DEPENDENCY_EVIDENCE_SOURCES = (
    (
        "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
        "B2 family catalog has no dependency edge field",
        "B2_FAMILY_CATALOG_NO_DEPENDENCY_GRAPH",
    ),
    (
        "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
        "B2 terminal assignments contain no reusable dependency edges",
        "B2_CLASSIFICATIONS_NO_DEPENDENCY_GRAPH",
    ),
    (
        "sources/m2_5/closures/B2/classification_closure.v2.json",
        "accepted B2 closure snapshot has no dependency graph",
        "B2_CLOSURE_NO_DEPENDENCY_GRAPH",
    ),
    (
        "sources/m2_5/closures/B2/B2_DESIGN_SPEC.md",
        "B2 terminal classification contract, not recursive capability closure",
        "B2_DESIGN_SCOPE_BOUNDARY",
    ),
    (
        "cards/capabilities/registry.json",
        "production capability registry is empty at this baseline",
        "PRODUCTION_CAPABILITY_REGISTRY_EMPTY",
    ),
)
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


def _validate_dependency_evidence_refs(
    evidence_refs: object,
    root: Path,
    *,
    expected_role: str | None = None,
    locator_prefix: str | None = None,
) -> list[dict[str, Any]]:
    _require(
        isinstance(evidence_refs, list) and evidence_refs,
        "dependency evidence is missing",
    )
    canonical: list[dict[str, Any]] = []
    for reference in evidence_refs:
        _require(isinstance(reference, dict), "dependency evidence reference is not an object")
        path_value = reference.get("path")
        raw_sha256 = reference.get("raw_sha256")
        locator = reference.get("locator")
        _require(
            isinstance(path_value, str) and path_value and isinstance(raw_sha256, str),
            "dependency evidence reference is incomplete",
        )
        relative = Path(path_value)
        _require(
            not relative.is_absolute() and ".." not in relative.parts,
            "dependency evidence path escapes repository",
        )
        evidence_path = root / relative
        _require(evidence_path.is_file(), f"dependency evidence file is missing: {path_value}")
        _require(
            _sha256(evidence_path.read_bytes()) == raw_sha256,
            f"dependency evidence digest mismatch: {path_value}",
        )
        _require(
            isinstance(locator, str) and locator,
            "dependency evidence locator is missing",
        )
        if expected_role is not None:
            _require(
                reference.get("evidence_role") == expected_role,
                f"dependency evidence role is not {expected_role}",
            )
        if locator_prefix is not None:
            _require(
                locator.startswith(locator_prefix),
                f"dependency evidence locator does not start with {locator_prefix}",
            )
        canonical.append(dict(reference))
    return sorted(canonical, key=lambda item: _canonical(item))


def validate_dependency_edges(
    edges: object,
    known_family_ids: set[str],
    root: Path = ROOT,
) -> list[dict[str, Any]]:
    """Validate and canonically order parent-requires-child dependency edges."""

    _require(isinstance(edges, list), "dependency edges must be an array")
    canonical: list[dict[str, Any]] = []
    by_pair: dict[tuple[str, str], dict[str, Any]] = {}
    allowed_keys = {
        "parent_family_id",
        "child_family_id",
        "dependency_kind",
        "evidence_refs",
        "rationale",
    }
    for edge in edges:
        _require(isinstance(edge, dict), "dependency edge is not an object")
        _require(
            set(edge) == allowed_keys,
            "dependency edge shape is not exact",
        )
        parent = edge["parent_family_id"]
        child = edge["child_family_id"]
        kind = edge["dependency_kind"]
        _require(
            isinstance(parent, str) and parent in known_family_ids,
            f"unknown parent capability family: {parent}",
        )
        _require(
            isinstance(child, str) and child in known_family_ids,
            f"unknown child capability family: {child}",
        )
        _require(parent != child, f"self-dependency is not permitted: {parent}")
        _require(
            isinstance(kind, str) and kind in DEPENDENCY_KINDS,
            f"unknown dependency kind: {kind}",
        )
        normalized = {
            "parent_family_id": parent,
            "child_family_id": child,
            "dependency_kind": kind,
            "evidence_refs": _validate_dependency_evidence_refs(
                edge["evidence_refs"],
                root,
                expected_role="ACCEPTED_DEPENDENCY_EDGE",
                locator_prefix="dependency-edge:",
            ),
            "rationale": edge["rationale"],
        }
        _require(
            isinstance(normalized["rationale"], str) and normalized["rationale"],
            "dependency rationale is missing",
        )
        pair = (parent, child)
        previous = by_pair.get(pair)
        if previous is not None:
            if previous == normalized:
                raise ScopeCensusValidationError(f"duplicate dependency edge: {parent} -> {child}")
            raise ScopeCensusValidationError(f"conflicting dependency edge: {parent} -> {child}")
        by_pair[pair] = normalized
        canonical.append(normalized)
    return sorted(
        canonical,
        key=lambda item: (
            item["parent_family_id"],
            item["child_family_id"],
            item["dependency_kind"],
            _canonical(item["evidence_refs"]),
            item["rationale"],
        ),
    )


def _canonical_cycle(cycle: list[str]) -> list[str]:
    ring = cycle[:-1]
    rotations = [ring[index:] + ring[:index] for index in range(len(ring))]
    best = min(rotations)
    return [*best, best[0]]


def compute_dependency_closure(
    direct_roots: list[str], edges: list[dict[str, Any]]
) -> dict[str, Any]:
    """Compute the deterministic parent-requires-child transitive closure."""

    _require(
        direct_roots == sorted(set(direct_roots)),
        "direct capability roots are not canonical or contain duplicates",
    )
    children: dict[str, list[str]] = defaultdict(list)
    nodes = set(direct_roots)
    for edge in edges:
        parent = edge["parent_family_id"]
        child = edge["child_family_id"]
        children[parent].append(child)
        nodes.update((parent, child))
    for parent in children:
        children[parent] = sorted(children[parent])

    state: dict[str, int] = {}
    stack: list[str] = []
    cycles: set[tuple[str, ...]] = set()

    def visit(node: str) -> None:
        marker = state.get(node, 0)
        if marker == 2:
            return
        if marker == 1:
            index = stack.index(node)
            cycles.add(tuple(_canonical_cycle([*stack[index:], node])))
            return
        state[node] = 1
        stack.append(node)
        for child in children.get(node, []):
            visit(child)
        stack.pop()
        state[node] = 2

    for node in sorted(nodes):
        visit(node)

    resolved: set[str] = set()
    pending = list(reversed(direct_roots))
    while pending:
        node = pending.pop()
        if node in resolved:
            continue
        resolved.add(node)
        pending.extend(reversed(children.get(node, [])))
    roots = set(direct_roots)
    return {
        "resolved_families": sorted(resolved),
        "transitive_only_families": sorted(resolved - roots),
        "cycles": [list(cycle) for cycle in sorted(cycles)],
    }


def compute_artifact_content_sha256(artifact: dict[str, Any]) -> str:
    value = copy.deepcopy(artifact)
    value.pop("content_sha256", None)
    return _sha256(_canonical(value))


def _artifact_serialized_bytes(artifact: dict[str, Any]) -> bytes:
    return (json.dumps(artifact, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def _binding(path: str, schema: str | None, raw_sha256: str) -> dict[str, Any]:
    return {"path": path, "schema": schema, "raw_sha256": raw_sha256}


def load_census_artifacts(root: Path = ROOT) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for kind, filename in ARTIFACTS.items():
        result[kind] = _json(root / SCOPE_RELATIVE_PATH / filename)
    return result


def _schema_for_artifact_kind(kind: str) -> tuple[Path, str]:
    if kind == "recursive_capability_closure":
        return CLOSURE_SCHEMA_RELATIVE_PATH, CLOSURE_SCHEMA_ID
    return CENSUS_SCHEMA_RELATIVE_PATH, CENSUS_SCHEMA_ID


def _validate_schema(artifact: dict[str, Any], root: Path) -> None:
    schema_path, _schema_id = _schema_for_artifact_kind(artifact["artifact_kind"])
    schema = _json(root / schema_path)
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


def _dependency_source_bindings(root: Path) -> list[dict[str, Any]]:
    bindings = []
    for path, locator, evidence_role in DEPENDENCY_EVIDENCE_SOURCES:
        file_path = root / path
        _require(file_path.is_file(), f"dependency evidence source is missing: {path}")
        bindings.append(
            {
                "path": path,
                "raw_sha256": _sha256(file_path.read_bytes()),
                "locator": locator,
                "evidence_role": evidence_role,
            }
        )
    return sorted(bindings, key=lambda item: item["path"])


def _dependency_evidence_ref(
    binding: dict[str, Any], *, b2_family_id: str | None = None
) -> dict[str, Any]:
    reference = dict(binding)
    if b2_family_id is not None:
        reference["b2_family_id"] = b2_family_id
    return reference


def _selected_capability_binding(root: Path, capability: dict[str, Any]) -> dict[str, Any]:
    path = root / SCOPE_RELATIVE_PATH / ARTIFACTS["capability"]
    return {
        "path": path.relative_to(root).as_posix(),
        "raw_sha256": _sha256(_artifact_serialized_bytes(capability)),
        "content_sha256": capability["content_sha256"],
        "source_package_sha256": EXPECTED_SOURCE_PACKAGE_SHA256,
    }


def _historical_closure_binding(root: Path) -> dict[str, Any]:
    path = root / HISTORICAL_CLOSURE_RELATIVE_PATH
    return {
        "path": HISTORICAL_CLOSURE_RELATIVE_PATH,
        "raw_sha256": _sha256(path.read_bytes()),
        "status": "HISTORICAL_BLOCKED_V1",
    }


def _semantic_owner_map(capability: dict[str, Any]) -> dict[str, list[str]]:
    owners = {
        family["family_id"]: sorted(set(family.get("semantic_owner_roles", [])))
        for family in capability["families"]
    }
    _require(
        all(owners[family_id] for family_id in owners),
        "selected capability family is missing semantic owner",
    )
    return dict(sorted(owners.items()))


def _closure_family_records(
    capability: dict[str, Any],
    catalog: dict[str, dict[str, Any]],
    family_ids: list[str],
) -> list[dict[str, Any]]:
    direct = {family["family_id"]: family for family in capability["families"]}
    records = []
    for family_id in sorted(family_ids):
        if family_id in direct:
            source = direct[family_id]
            family = catalog[family_id]
            records.append(
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
                    "edge_count": source["edge_count"],
                    "oracle_identity_count": source["oracle_identity_count"],
                    "deck_row_count": source["deck_row_count"],
                }
            )
            continue
        family = catalog.get(family_id)
        _require(family is not None, f"resolved family is absent from B2 catalog: {family_id}")
        records.append(
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
            }
        )
    return records


def _validate_capability_family_metadata(
    capability: dict[str, Any], catalog: dict[str, dict[str, Any]]
) -> None:
    assignment_counts: dict[str, dict[str, int]] = defaultdict(
        lambda: {"edge_count": 0, "oracle_identity_count": 0, "deck_row_count": 0}
    )
    for record in capability["records"]:
        assignments = record["capability_assignments"]
        for assignment in assignments:
            family_id = assignment["family_id"]
            counts = assignment_counts[family_id]
            counts["edge_count"] += len(record["deck_row_ids"])
            counts["oracle_identity_count"] += 1
            counts["deck_row_count"] += len(record["deck_row_ids"])
    for family in capability["families"]:
        family_id = family["family_id"]
        catalog_family = catalog.get(family_id)
        _require(
            catalog_family is not None, f"capability family is absent from B2 catalog: {family_id}"
        )
        expected = {
            "family_id": family_id,
            "canonical_name": catalog_family["canonical_name"],
            "status": catalog_family["status"],
            "terminal_assignable": catalog_family["terminal_assignable"],
            "precise_semantic_definition_sha256": _sha256(
                catalog_family["precise_semantic_definition"].encode("utf-8")
            ),
            "semantic_owner_roles": _family_owner_roles(catalog_family),
            "semantic_owner_status": "SCOPE_ROLE_REQUIRED",
            **assignment_counts[family_id],
        }
        _require(
            all(family.get(key) == value for key, value in expected.items()),
            f"selected capability family metadata is not B2-bound: {family_id}",
        )


def _closure_owner_map(
    capability: dict[str, Any],
    catalog: dict[str, dict[str, Any]],
    family_ids: list[str],
) -> dict[str, list[str]]:
    return {
        record["family_id"]: sorted(record["semantic_owner_roles"])
        for record in _closure_family_records(capability, catalog, family_ids)
    }


def _root_dependency_evidence(
    root: Path,
    family_id: str,
    capability_binding: dict[str, Any],
    dependency_bindings: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    references = [
        {
            "path": capability_binding["path"],
            "raw_sha256": capability_binding["raw_sha256"],
            "locator": f"families/{family_id}",
            "evidence_role": "SELECTED_DIRECT_CAPABILITY_ROOT",
            "source_package_sha256": EXPECTED_SOURCE_PACKAGE_SHA256,
            "b2_family_id": family_id,
        }
    ]
    references.extend(
        _dependency_evidence_ref(binding, b2_family_id=family_id) for binding in dependency_bindings
    )
    return sorted(references, key=lambda item: _canonical(item))


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
    _schema_path, schema_id = _schema_for_artifact_kind(kind)
    return {
        "schema": schema_id,
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


def _selected_osis_for_families(
    capability: dict[str, Any], family_ids: tuple[str, ...]
) -> list[str]:
    wanted = set(family_ids)
    return sorted(
        {
            record["oracle_semantic_identity"]
            for record in capability["records"]
            if any(
                assignment["family_id"] in wanted for assignment in record["capability_assignments"]
            )
        }
    )


def _capability_assignment_evidence(
    capability: dict[str, Any], source_osis: list[str], family_ids: tuple[str, ...]
) -> list[dict[str, Any]]:
    selected_osis = set(source_osis)
    wanted = set(family_ids)
    evidence = []
    for record in capability["records"]:
        osi = record["oracle_semantic_identity"]
        if osi not in selected_osis:
            continue
        for assignment in record["capability_assignments"]:
            if assignment["family_id"] not in wanted:
                continue
            evidence.append(
                {
                    "kind": "B2_CAPABILITY_ASSIGNMENT",
                    "artifact_path": B2_FILES["classifications"][0],
                    "source_package_sha256": EXPECTED_SOURCE_PACKAGE_SHA256,
                    "oracle_semantic_identity": osi,
                    "family_id": assignment["family_id"],
                    "classification_identity_sha256": assignment["classification_identity_sha256"],
                    "review_status": assignment["review_status"],
                    "evidence_basis": assignment["evidence_basis"],
                    "assignment_evidence_sha256": assignment["assignment_evidence_sha256"],
                    "source_evidence_digest": assignment["source_evidence_digest"],
                    "family_boundary_sha256": assignment["family_boundary_sha256"],
                }
            )
    return sorted(
        evidence,
        key=lambda item: (item["oracle_semantic_identity"], item["family_id"]),
    )


def _scope_contract_evidence(
    root: Path, contracts: tuple[tuple[str, str], ...]
) -> list[dict[str, Any]]:
    return [
        {
            "kind": "SCOPE_CONTRACT",
            "path": path,
            "raw_sha256": _sha256((root / path).read_bytes()),
            "locator": locator,
        }
        for path, locator in contracts
    ]


def _emblem_negative_scan_evidence(
    context: dict[str, Any], capability: dict[str, Any]
) -> list[dict[str, Any]]:
    emblem_part_count = 0
    for osi in context["osis"]:
        source_record_id = context["rows_by_osi"][osi][0]["oracle_source_record_id"]
        source_record = context["archive"]["oracle_records"][source_record_id]
        emblem_part_count += sum(
            part.get("component") == "emblem" for part in source_record.get("all_parts") or []
        )
    b2_emblem_assignment_count = sum(
        assignment["family_id"] == "cap.emblem"
        for record in capability["records"]
        for assignment in record["capability_assignments"]
    )
    oracle_member = "source/raw/oracle_cards_selected_REV3.jsonl"
    return [
        {
            "kind": "REV3_SELECTED_ALL_PARTS_NEGATIVE_SCAN",
            "artifact_path": oracle_member,
            "archive_member_sha256": context["archive"]["entries"][oracle_member]["sha256"],
            "source_package_sha256": EXPECTED_SOURCE_PACKAGE_SHA256,
            "selected_oracle_identity_count": len(context["osis"]),
            "emblem_part_count": emblem_part_count,
            "b2_emblem_assignment_count": b2_emblem_assignment_count,
        }
    ]


def _build_semantic_generated_object_records(
    context: dict[str, Any], capability: dict[str, Any]
) -> list[dict[str, Any]]:
    records = []
    for class_id, spec in SEMANTIC_GENERATED_OBJECT_SPECS.items():
        record = {
            "object_class_id": class_id,
            "object_kind": spec["object_kind"],
            "status": spec["status"],
            "source_binding_mode": spec["binding_mode"],
        }
        if spec["binding_mode"] == "B2_ASSIGNMENT":
            source_osis = _selected_osis_for_families(
                capability, spec["source_selector_family_ids"]
            )
            evidence = _capability_assignment_evidence(
                capability, source_osis, spec["evidence_family_ids"]
            )
            _require(source_osis, f"missing semantic generated-object source OSIs: {class_id}")
            _require(evidence, f"missing semantic generated-object capability evidence: {class_id}")
            record.update(
                {
                    "source_card_osis": source_osis,
                    "owning_capability_families": sorted({item["family_id"] for item in evidence}),
                    "capability_evidence": evidence,
                    "evidence_refs": evidence,
                }
            )
        elif spec["binding_mode"] == "SCOPE_INVARIANT":
            evidence = _scope_contract_evidence(context["root"], spec["scope_contracts"])
            record.update(
                {
                    "source_card_osis": [],
                    "owning_capability_families": [],
                    "capability_evidence": [],
                    "evidence_refs": evidence,
                }
            )
        else:
            _require(
                spec["binding_mode"] == "REV3_NEGATIVE_SCAN",
                f"unknown semantic generated-object binding mode: {class_id}",
            )
            record.update(
                {
                    "source_card_osis": [],
                    "owning_capability_families": [],
                    "capability_evidence": [],
                    "evidence_refs": _emblem_negative_scan_evidence(context, capability),
                }
            )
        records.append(record)
    return records


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


def _build_generated_objects(context: dict[str, Any], capability: dict[str, Any]) -> dict[str, Any]:
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
    semantic = _build_semantic_generated_object_records(context, capability)
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
    family_records = _closure_family_records(capability, context["catalog"], roots)
    capability_binding = _selected_capability_binding(context["root"], capability)
    dependency_bindings = _dependency_source_bindings(context["root"])
    owner_map = _closure_owner_map(capability, context["catalog"], roots)
    root_classifications = []
    for family_id in roots:
        evidence_refs = _root_dependency_evidence(
            context["root"], family_id, capability_binding, dependency_bindings
        )
        root_classifications.append(
            {
                "family_id": family_id,
                "state": "BLOCKED_MISSING_DEPENDENCY_EVIDENCE",
                "semantic_owner_roles": owner_map[family_id],
                "evidence_refs": evidence_refs,
                "reason_code": "NO_ACCEPTED_REUSABLE_DEPENDENCY_EVIDENCE",
                "future_owner": owner_map[family_id][0],
            }
        )
    unresolved = [
        {
            "obligation_id": f"recursive-dependency:{record['family_id']}",
            "reason_code": record["reason_code"],
            "subject": record["family_id"],
            "evidence_refs": record["evidence_refs"],
            "future_owner": record["future_owner"],
        }
        for record in root_classifications
    ]
    artifact.update(
        {
            "status": "BLOCKED",
            "selected_capability_census": capability_binding,
            "superseded_artifact": _historical_closure_binding(context["root"]),
            "dependency_direction": (
                "parent_family_id -> child_family_id means supporting the parent requires the child"
            ),
            "dependency_evidence_bindings": dependency_bindings,
            "accepted_terminal_leaf_evidence": [],
            "direct_roots": roots,
            "resolved_families": roots,
            "transitive_only_families": [],
            "dependency_edges": [],
            "terminal_leaves": [],
            "blocked_families": roots,
            "missing_capabilities": [],
            "cycles": [],
            "root_classifications": root_classifications,
            "semantic_owner_map": owner_map,
            "families": family_records,
            "records": root_classifications,
            "unresolved_scope_obligations": unresolved,
        }
    )
    artifact["record_counts"] = {
        "direct_capability_roots": len(roots),
        "resolved_recursive_capability_families": len(roots),
        "transitive_only_capability_families": 0,
        "explicit_dependency_edges": 0,
        "terminal_leaves": 0,
        "blocked_families": len(roots),
        "missing_capabilities": 0,
        "cycles": 0,
        "unresolved_dependency_obligations": len(roots),
    }
    artifact["high_risk_outliers"] = capability["high_risk_outliers"]
    return artifact


def build_census_artifacts(
    root: Path = ROOT, archive_root: Path | None = None
) -> dict[str, dict[str, Any]]:
    configured = archive_root or Path(os.environ[ARCHIVE_ENV_VAR])
    context = _build_context(root, configured)
    capability = _build_capability(context)
    validate_b2_projection_rows(context["projection"], context["classifications"], capability)
    capability["content_sha256"] = compute_artifact_content_sha256(capability)
    decision = _build_surface_artifact(
        context, "decision", DECISION_SURFACE_MAP, "decision_surface"
    )
    information = _build_surface_artifact(
        context, "information", INFO_SURFACE_MAP, "information_surface"
    )
    generated = _build_generated_objects(context, capability)
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
    artifact: dict[str, Any],
    root: Path,
    archive_root: Path,
    capability: dict[str, Any] | None = None,
) -> None:
    """Verify generated/reference records against exact source and B2 evidence."""

    context = _build_context(root, archive_root)
    lock = context["lock"]
    osis = set(context["osis"])
    archive = context["archive"]
    if capability is None:
        capability = _json(root / SCOPE_RELATIVE_PATH / ARTIFACTS["capability"])
    _require(
        capability["content_sha256"] == compute_artifact_content_sha256(capability),
        "capability artifact content digest mismatch",
    )
    validate_b2_projection_rows(context["projection"], context["classifications"], capability)
    expected_semantic = {
        record["object_class_id"]: record
        for record in _build_semantic_generated_object_records(context, capability)
    }
    source_by_osi = {
        row["oracle_semantic_identity"]: row for deck in lock["decks"] for row in deck["cards"]
    }
    seen_semantic: set[str] = set()
    for record in artifact["records"]:
        source_osis = set(record.get("source_card_osis", []))
        _require(source_osis.issubset(osis), "generated object has unbound source identity")
        if record["object_kind"] in {"TOKEN_CLASS", "REFERENCED_OBJECT"}:
            continue
        class_id = record.get("object_class_id")
        _require(
            class_id in expected_semantic,
            f"unknown semantic generated-object class: {class_id}",
        )
        _require(
            class_id not in seen_semantic, f"duplicate semantic generated-object class: {class_id}"
        )
        seen_semantic.add(class_id)
        expected = expected_semantic[class_id]
        for field, label in (
            ("object_kind", "kind"),
            ("status", "status"),
            ("source_binding_mode", "source binding mode"),
            ("source_card_osis", "source identity evidence"),
            ("owning_capability_families", "owning capability families"),
            ("capability_evidence", "capability evidence"),
            ("evidence_refs", "evidence references"),
        ):
            _require(
                record.get(field) == expected[field],
                f"semantic generated object {label} mismatch: {class_id}",
            )
    _require(
        seen_semantic == set(expected_semantic),
        "semantic generated-object class set is incomplete",
    )
    for record in artifact["records"]:
        if record["object_kind"] not in {"TOKEN_CLASS", "REFERENCED_OBJECT"}:
            continue
        source_osis = set(record.get("source_card_osis", []))
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


def validate_recursive_capability_closure(
    artifact: dict[str, Any],
    capability: dict[str, Any],
    root: Path = ROOT,
) -> None:
    """Validate the v2 recursive closure without inferring missing edges."""

    _validate_schema(artifact, root)
    _require(
        artifact["content_sha256"] == compute_artifact_content_sha256(artifact),
        "recursive closure content digest mismatch",
    )
    _require(
        artifact["source_package_sha256"] == EXPECTED_SOURCE_PACKAGE_SHA256,
        "recursive closure source package binding mismatch",
    )
    _require(
        capability["content_sha256"] == compute_artifact_content_sha256(capability),
        "capability census content digest mismatch",
    )
    family_ids = {family["family_id"] for family in capability["families"]}
    catalog = {
        family["family_id"]: family
        for family in _json(root / B2_FILES["family_catalog"][0])["families"]
    }
    _validate_capability_family_metadata(capability, catalog)
    all_family_ids = set(catalog)
    roots = sorted(family_ids)
    _require(artifact["direct_roots"] == roots, "recursive closure roots are not exact")
    _require(
        artifact["selected_pair"] == capability["selected_pair"],
        "recursive closure selected-pair binding mismatch",
    )
    _require(
        artifact["selected_capability_census"]["path"]
        == (SCOPE_RELATIVE_PATH / ARTIFACTS["capability"]).as_posix(),
        "recursive closure capability census path mismatch",
    )
    capability_path = root / artifact["selected_capability_census"]["path"]
    _require(
        capability_path.is_file()
        and _sha256(capability_path.read_bytes())
        == artifact["selected_capability_census"]["raw_sha256"],
        "recursive closure capability census raw digest mismatch",
    )
    _require(
        artifact["selected_capability_census"]["content_sha256"] == capability["content_sha256"],
        "recursive closure capability census content binding mismatch",
    )
    _require(
        artifact["selected_capability_census"]["source_package_sha256"]
        == EXPECTED_SOURCE_PACKAGE_SHA256,
        "recursive closure capability census source package mismatch",
    )
    _require(
        artifact["superseded_artifact"] == _historical_closure_binding(root),
        "recursive closure historical v1 binding mismatch",
    )
    _require(
        artifact["dependency_evidence_bindings"] == _dependency_source_bindings(root),
        "recursive closure dependency evidence bindings drift",
    )
    _require(
        not artifact["accepted_terminal_leaf_evidence"],
        "accepted terminal-leaf evidence contract is unavailable",
    )
    edges = validate_dependency_edges(artifact["dependency_edges"], all_family_ids, root)
    _require(
        artifact["dependency_edges"] == edges,
        "recursive closure dependency edge ordering or evidence drift",
    )
    closure = compute_dependency_closure(roots, edges)
    _require(
        set(closure["resolved_families"]).issubset(all_family_ids),
        "recursive closure resolved an unknown B2 family",
    )
    owner_map = _closure_owner_map(capability, catalog, closure["resolved_families"])
    _require(
        artifact["semantic_owner_map"] == owner_map,
        "recursive closure semantic owner map mismatch",
    )
    _require(
        artifact["families"]
        == _closure_family_records(capability, catalog, closure["resolved_families"]),
        "recursive closure family records are not source-bound",
    )
    _require(
        artifact["resolved_families"] == closure["resolved_families"],
        "recursive closure resolved family set mismatch",
    )
    _require(
        artifact["transitive_only_families"] == closure["transitive_only_families"],
        "recursive closure transitive family set mismatch",
    )
    _require(
        artifact["cycles"] == closure["cycles"],
        "recursive closure cycle set mismatch",
    )

    classifications = artifact["root_classifications"]
    _require(
        isinstance(classifications, list)
        and [record.get("family_id") for record in classifications] == roots,
        "recursive closure root classifications are not exact",
    )
    _require(
        artifact["records"] == classifications,
        "recursive closure records are not reconciled with root classifications",
    )
    expected_root_evidence = _selected_capability_binding(root, capability)
    expected_dependency_bindings = _dependency_source_bindings(root)
    for record in classifications:
        _require(
            record["evidence_refs"]
            == _root_dependency_evidence(
                root,
                record["family_id"],
                expected_root_evidence,
                expected_dependency_bindings,
            ),
            f"recursive closure root evidence is not source-reconciled: {record['family_id']}",
        )
    outgoing = defaultdict(list)
    for edge in edges:
        outgoing[edge["parent_family_id"]].append(edge)
    declared_blocked = set(artifact["blocked_families"])
    blocked: set[str] = set()
    terminal: set[str] = set()
    dependency_roots: set[str] = set()
    for record in classifications:
        family_id = record["family_id"]
        state = record["state"]
        owners = record["semantic_owner_roles"]
        _require(
            owners == owner_map[family_id] and owners,
            f"recursive closure semantic owner missing: {family_id}",
        )
        evidence = _validate_dependency_evidence_refs(record["evidence_refs"], root)
        _require(
            evidence == record["evidence_refs"],
            f"recursive closure evidence ordering drift: {family_id}",
        )
        if state == "TERMINAL_LEAF":
            _require(
                family_id not in declared_blocked,
                "family is both terminal and blocked",
            )
            raise ScopeCensusValidationError(
                "accepted terminal-leaf evidence contract is unavailable"
            )
            _require(
                not outgoing.get(family_id),
                f"terminal leaf has accepted dependencies: {family_id}",
            )
            terminal.add(family_id)
        elif state == "HAS_ACCEPTED_DEPENDENCIES":
            _require(
                outgoing.get(family_id),
                f"accepted dependency state has no edges: {family_id}",
            )
            dependency_roots.add(family_id)
        elif state == "BLOCKED_MISSING_DEPENDENCY_EVIDENCE":
            _require(
                record["reason_code"] == "NO_ACCEPTED_REUSABLE_DEPENDENCY_EVIDENCE",
                f"blocked dependency reason is not exact: {family_id}",
            )
            _require(
                record["future_owner"] in owners,
                f"blocked dependency future owner is not exact: {family_id}",
            )
            blocked.add(family_id)
        else:
            raise ScopeCensusValidationError(f"unknown recursive closure root state: {state}")

    _require(
        not (terminal & declared_blocked),
        "family is both terminal and blocked",
    )
    _require(not (terminal & blocked), "family is both terminal and blocked")
    _require(not (terminal & dependency_roots), "family is both terminal and dependency root")
    _require(not (blocked & dependency_roots), "family is both blocked and dependency root")
    _require(
        artifact["terminal_leaves"] == sorted(terminal),
        "recursive closure terminal leaf set mismatch",
    )
    _require(
        artifact["blocked_families"] == sorted(blocked),
        "recursive closure blocked family set mismatch",
    )
    for missing in artifact["missing_capabilities"]:
        _require(
            missing["parent_family_id"] in all_family_ids,
            f"missing capability has unknown parent: {missing['parent_family_id']}",
        )
        _require(
            missing["future_owner"] in owner_map[missing["parent_family_id"]],
            f"missing capability future owner is not exact: {missing['required_key']}",
        )
        _validate_dependency_evidence_refs(missing["evidence_refs"], root)
    if artifact["status"] == "PASS":
        _require(not blocked, "recursive closure PASS has blocked roots")
        _require(
            not artifact["missing_capabilities"], "recursive closure PASS has missing capabilities"
        )
        _require(not closure["cycles"], "recursive closure PASS has cycles")
        _require(
            not artifact["unresolved_scope_obligations"],
            "recursive closure PASS has unresolved obligations",
        )
    expected_unresolved = [
        {
            "obligation_id": f"recursive-dependency:{record['family_id']}",
            "reason_code": record["reason_code"],
            "subject": record["family_id"],
            "evidence_refs": record["evidence_refs"],
            "future_owner": record["future_owner"],
        }
        for record in classifications
        if record["state"] == "BLOCKED_MISSING_DEPENDENCY_EVIDENCE"
    ]
    expected_unresolved.extend(
        {
            "obligation_id": (
                f"missing-capability:{missing['required_key']}:{missing['parent_family_id']}"
            ),
            "reason_code": "MISSING_CAPABILITY_REQUIREMENT",
            "subject": missing["required_key"],
            "evidence_refs": missing["evidence_refs"],
            "future_owner": missing["future_owner"],
        }
        for missing in artifact["missing_capabilities"]
    )
    expected_unresolved.sort(key=lambda item: item["obligation_id"])
    _require(
        artifact["unresolved_scope_obligations"] == expected_unresolved,
        "recursive closure unresolved obligation evidence mismatch",
    )
    expected_counts = {
        "direct_capability_roots": len(roots),
        "resolved_recursive_capability_families": len(closure["resolved_families"]),
        "transitive_only_capability_families": len(closure["transitive_only_families"]),
        "explicit_dependency_edges": len(edges),
        "terminal_leaves": len(terminal),
        "blocked_families": len(blocked),
        "missing_capabilities": len(artifact["missing_capabilities"]),
        "cycles": len(closure["cycles"]),
        "unresolved_dependency_obligations": len(artifact["unresolved_scope_obligations"]),
    }
    _require(
        artifact["record_counts"] == expected_counts,
        "recursive closure record counts mismatch",
    )
    if artifact["status"] != "PASS":
        _require(
            artifact["status"] == "BLOCKED",
            f"unknown recursive closure status: {artifact['status']}",
        )
        _require(
            blocked or artifact["missing_capabilities"] or closure["cycles"],
            "recursive closure BLOCKED has no blocking reason",
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
    validate_recursive_capability_closure(closure, capability, root)
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
        validate_generated_object_records(
            artifacts["generated_object"], root, archive_root, capability=capability
        )


def _write_artifacts(artifacts: dict[str, dict[str, Any]], root: Path) -> None:
    target = root / SCOPE_RELATIVE_PATH
    target.mkdir(parents=True, exist_ok=True)
    for kind, filename in ARTIFACTS.items():
        (target / filename).write_bytes(_artifact_serialized_bytes(artifacts[kind]))


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
