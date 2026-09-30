#!/usr/bin/env python3
"""Generate semantic-contract catalog constants and V6 execution profiles (spec §10).

Single generator for the single hand-authored manifest source
``contracts/catalog/semantic-contracts.v1.json``. Emits BOTH the Rust manifest
constants AND the derived ``RulesContractIdV1``/``SemanticContractIdV1``
values into ``crates/mtgml-environment/src/semantic_catalog_generated.rs``.

Layering (review Fix-02):

- ``render_catalog_generated`` is a pure renderer of VALID manifest facts.
  It derives IDs exclusively via the Task-2 Python mechanical mirrors (no
  digest logic of its own) and can render any manifest the §7 contract
  accepts.
- ``assert_production_policy`` is the PRODUCTION source validator.
- The CLI (``main``) always enforces the production policy before rendering
  and checks the target byte-exactly on ``--check``.

Generated-file policy (Batch G FND-030): output is written as exact UTF-8/LF
bytes so byte comparison is host-independent.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# The project Python root is added so the Task-2 mirrors can be imported from
# a bare checkout without requiring an editable install.
sys.path.insert(0, str(ROOT / "python" / "src"))

SOURCE_REL = "contracts/catalog/semantic-contracts.v1.json"
SOURCE_PATH = ROOT / SOURCE_REL
TARGET_REL = "crates/mtgml-environment/src/semantic_catalog_generated.rs"
TARGET_PATH = ROOT / TARGET_REL

RULES_ENTRY_ID = "magic_turn_structure_0_1_0"
S3A_ENTRY_ID = "magic_s3_a_ordered_sba_0_1_0"
S3B_ENTRY_ID = "magic_s3_b_basic_priority_0_1_0"
S3C_ENTRY_ID = "magic_s3_c_draw_interaction_0_1_0"
COMBAT_ENTRY_ID = "magic_combat_attackers_0_1_0"
COMBAT_BLOCKERS_ENTRY_ID = "magic_combat_blockers_0_1_0"
COMBAT_DAMAGE_ENTRY_ID = "magic_combat_damage_0_1_0"
BOUNDED_TURN_ENTRY_ID = "magic_bounded_turn_0_1_0"


FACT_KEYS = {
    "entry_id",
    "rules_authority",
    "capability_closure",
    "format_contract_id",
    "content_contract_id",
}


def load_source(path: Path | None = None) -> dict[str, object]:
    source = path if path is not None else SOURCE_PATH
    data = json.loads(source.read_text(encoding="utf-8"))
    if data.get("schema_version") != "semantic-contracts-catalog.v1":
        raise SystemExit("unsupported semantic-contracts catalog")
    entries = data.get("entries")
    if not isinstance(entries, list) or not entries:
        raise SystemExit("semantic-contracts catalog must contain entries")
    return data


def validate_entry_facts(entry: dict[str, object]) -> None:
    """The source carries manifest FACTS only; derived identity is generated
    output and must never appear in the hand-authored source (no second
    authority for identity)."""
    if set(entry) != FACT_KEYS:
        raise SystemExit(
            f"catalog entry keys must be exactly {sorted(FACT_KEYS)}; got {sorted(entry)}"
        )


def rust_string_literal(value: object, label: str) -> str:
    """Render a plain double-quoted Rust string literal, fail-closed for any
    character this generator does not represent."""
    if (
        not isinstance(value, str)
        or not value
        or not value.isascii()
        or '"' in value
        or "\\" in value
    ):
        raise SystemExit(f"{label} is not representable as a plain Rust string literal")
    return f'"{value}"'


def derive_ids(entry: dict[str, object]) -> tuple[str, str]:
    """Derive both IDs via the Task-2 Python mechanical mirrors — the ONLY
    digest path; this generator contains no digest logic of its own. Invalid
    manifest facts fail closed here, before any emission."""
    from mtgml.persistence import (
        calculate_rules_contract_id_v1,
        calculate_semantic_contract_id_v1,
    )

    rules_id = calculate_rules_contract_id_v1(
        {
            "rules_authority": entry["rules_authority"],
            "capability_closure": entry["capability_closure"],
        }
    )
    semantic_id = calculate_semantic_contract_id_v1(
        {
            "rules_contract_id": rules_id,
            "format_contract_id": entry["format_contract_id"],
            "content_contract_id": entry["content_contract_id"],
        }
    )
    return rules_id, semantic_id


def render_rules_authority(authority: object) -> list[str]:
    if not isinstance(authority, dict):
        raise SystemExit("rules_authority must be an object")
    variant = authority.get("variant")
    if variant == "synthetic_legacy":
        if set(authority) != {"variant"}:
            raise SystemExit("synthetic_legacy authority carries no payload fields")
        return ["        rules_authority: mtgml_model::RulesAuthorityV1::SyntheticLegacy,"]
    if variant == "comprehensive_rules":
        if set(authority) != {"variant", "snapshot_id"}:
            raise SystemExit("comprehensive_rules authority carries exactly a snapshot_id")
        snapshot = rust_string_literal(authority["snapshot_id"], "comprehensive snapshot_id")
        return [
            "        rules_authority: mtgml_model::RulesAuthorityV1::ComprehensiveRules {",
            f"            snapshot_id: {snapshot}.to_owned(),",
            "        },",
        ]
    raise SystemExit(f"unknown rules authority variant: {variant!r}")


def render_closure(closure: object) -> list[str]:
    if closure is None:
        return ["        capability_closure: None,"]
    if not isinstance(closure, list) or not closure:
        raise SystemExit("capability_closure must be null or a non-empty list")
    lines = ["        capability_closure: Some(vec!["]
    for item in closure:
        if not isinstance(item, dict) or set(item) != {"key", "version"}:
            raise SystemExit("capability entry must carry exactly key and version")
        key = rust_string_literal(item["key"], "capability key")
        version = rust_string_literal(item["version"], "capability version")
        lines.append("            mtgml_model::CapabilityRequirementV1 {")
        lines.append(f"                key: {key}.to_owned(),")
        lines.append(f"                version: {version}.to_owned(),")
        lines.append("            },")
    lines.append("        ]),")
    return lines


def parse_call_lines(type_name: str, const_name: str) -> list[str]:
    """Emit a rustfmt-canonical ``Type::parse(CONST)`` expression.

    rustfmt (max_width 100) keeps the call on one line when it fits and
    otherwise breaks the argument with a trailing comma and re-indents the
    following ``.expect``. The generator reproduces both forms exactly so the
    emitted file survives ``cargo fmt --check`` byte-identically.
    """
    single = f"    {type_name}::parse({const_name})"
    if len(single) <= 100:
        return [single, '        .expect("generated canonical hex")']
    return [
        f"    {type_name}::parse(",
        f"        {const_name},",
        "    )",
        '    .expect("generated canonical hex")',
    ]


def render_catalog_generated(catalog: dict[str, object] | None = None) -> str:
    catalog = catalog if catalog is not None else load_source()
    lines: list[str] = []
    lines.append("// @generated by scripts/generate_semantic_contract_catalog.py; DO NOT EDIT.")
    lines.append(f"// Source: {SOURCE_REL}")
    lines.append("//")
    lines.append("// Manifest constants AND derived ID values for the runtime semantic")
    lines.append("// catalog (spec §10). Rust const-evaluation cannot allocate, so the")
    lines.append("// derived ID VALUES are checked in as canonical hex string constants")
    lines.append("// and materialized deterministically through parse() accessors (no")
    lines.append("// lazy statics — spec §10 bans hidden process state). The recompute KAT")
    lines.append("// (semantic_catalog_kat.rs) re-derives every ID from these generated")
    lines.append("// manifest constants via the §9 persistence functions and fails the")
    lines.append("// build on drift. Hand-editing any constant is a gate violation.")
    lines.append(
        "#![allow(dead_code)] // hex constants and *_rules_contract_id accessors are KAT/test-only"
    )
    lines.append("")
    lines.append("use mtgml_model::{RulesContractIdV1, SemanticContractIdV1};")
    lines.append("")
    for entry in catalog["entries"]:
        if not isinstance(entry, dict):
            raise SystemExit("catalog entry must be an object")
        validate_entry_facts(entry)
        entry_id = entry["entry_id"]
        if not isinstance(entry_id, str) or not entry_id:
            raise SystemExit("catalog entry_id must be a non-empty string")
        if entry_id != entry_id.replace("-", "_"):
            raise SystemExit("catalog entry_id must not contain hyphens")
        if entry["format_contract_id"] is not None or entry["content_contract_id"] is not None:
            raise SystemExit(
                f"catalog entry {entry_id!r}: format/content dimensions must be null "
                "in the V5 slice"
            )
        rules_id, semantic_id = derive_ids(entry)
        snake = entry_id.replace("-", "_")
        const_prefix = f"SEMANTIC_CONTRACT_CATALOG_{snake.upper()}"
        lines.append(f"// {entry_id}")
        lines.append(f"pub const {const_prefix}_RULES_CONTRACT_HEX: &str =")
        lines.append(f'    "{rules_id}";')
        lines.append(f"pub const {const_prefix}_SEMANTIC_CONTRACT_HEX: &str =")
        lines.append(f'    "{semantic_id}";')
        lines.append("")
        # GENERATED MANIFEST FACTS: the manifest constructors are emitted from
        # the source facts themselves, so the recompute KAT builds its
        # manifests exclusively from GENERATED data — no hand-copied manifest
        # anywhere (single-source-of-truth chain, spec §10).
        lines.append(f"pub fn {snake}_rules_manifest() -> mtgml_model::RulesContractManifestV1 {{")
        lines.append("    mtgml_model::RulesContractManifestV1 {")
        lines.extend(render_rules_authority(entry["rules_authority"]))
        lines.extend(render_closure(entry["capability_closure"]))
        lines.append("    }")
        lines.append("}")
        lines.append("")
        semantic_manifest_signature = f"pub fn {snake}_semantic_manifest"
        semantic_manifest_return = "mtgml_model::SemanticContractManifestV1"
        semantic_manifest_head = f"{semantic_manifest_signature}() -> {semantic_manifest_return}"
        if len(f"{semantic_manifest_head} {{") <= 100:
            lines.append(f"{semantic_manifest_head} {{")
        elif len(semantic_manifest_head) <= 101:
            lines.append(semantic_manifest_head)
            lines.append("{")
        else:
            lines.append(f"{semantic_manifest_signature}(")
            lines.append(f") -> {semantic_manifest_return} {{")
        lines.append("    mtgml_model::SemanticContractManifestV1 {")
        lines.append(f"        rules_contract_id: {snake}_rules_contract_id(),")
        lines.append("        format_contract_id: None,")
        lines.append("        content_contract_id: None,")
        lines.append("    }")
        lines.append("}")
        lines.append("")
        lines.append(f"pub fn {snake}_rules_contract_id() -> RulesContractIdV1 {{")
        lines.extend(
            parse_call_lines(
                "RulesContractIdV1",
                f"{const_prefix}_RULES_CONTRACT_HEX",
            )
        )
        lines.append("}")
        lines.append("")
        lines.append(f"pub fn {snake}_semantic_contract_id() -> SemanticContractIdV1 {{")
        lines.extend(
            parse_call_lines(
                "SemanticContractIdV1",
                f"{const_prefix}_SEMANTIC_CONTRACT_HEX",
            )
        )
        lines.append("}")
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def assert_production_policy(catalog: dict[str, object]) -> None:
    """Require Synthetic, frozen S1, and distinct bounded S3.A/S3.B identities."""
    entries = catalog["entries"]
    if len(entries) != 9:
        raise SystemExit(
            "production semantic-contract catalog must contain exactly nine entries; "
            f"got {len(entries)}"
        )
    by_id = {}
    for entry in entries:
        if not isinstance(entry, dict):
            raise SystemExit("catalog entry must be an object")
        validate_entry_facts(entry)
        entry_id = entry["entry_id"]
        if entry_id in by_id:
            raise SystemExit(f"duplicate production semantic entry {entry_id!r}")
        by_id[entry_id] = entry
    synthetic = by_id.get("synthetic_legacy_default")
    turn_structure_entry = by_id.get(RULES_ENTRY_ID)
    ordered_sba_entry = by_id.get(S3A_ENTRY_ID)
    basic_priority_entry = by_id.get(S3B_ENTRY_ID)
    draw_entry = by_id.get(S3C_ENTRY_ID)
    combat_entry = by_id.get(COMBAT_ENTRY_ID)
    combat_blockers_entry = by_id.get(COMBAT_BLOCKERS_ENTRY_ID)
    combat_damage_entry = by_id.get(COMBAT_DAMAGE_ENTRY_ID)
    bounded_turn_entry = by_id.get(BOUNDED_TURN_ENTRY_ID)
    if (
        synthetic is None
        or turn_structure_entry is None
        or ordered_sba_entry is None
        or basic_priority_entry is None
        or draw_entry is None
        or combat_entry is None
        or combat_blockers_entry is None
        or combat_damage_entry is None
        or bounded_turn_entry is None
    ):
        raise SystemExit(
            "production semantic catalog requires Synthetic, exact S1, S3.A, S3.B, S3.C, "
            "combat, combat+blockers, combat+damage, and bounded-turn entries"
        )
    if (
        synthetic["rules_authority"] != {"variant": "synthetic_legacy"}
        or synthetic["capability_closure"] is not None
        or synthetic["format_contract_id"] is not None
        or synthetic["content_contract_id"] is not None
    ):
        raise SystemExit("synthetic production entry must retain its exact legacy identity")
    expected_authority = {
        "variant": "comprehensive_rules",
        "snapshot_id": (
            "wotc-cr-2026-08-07-txt-20260819-sha256-"
            "4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f"
        ),
    }
    if turn_structure_entry["rules_authority"] != expected_authority or turn_structure_entry[
        "capability_closure"
    ] != [{"key": "rules/turn-structure", "version": "0.1.0"}]:
        raise SystemExit("S1 must preserve the exact historical turn-structure-only closure")
    if ordered_sba_entry["rules_authority"] != expected_authority or ordered_sba_entry[
        "capability_closure"
    ] != [
        {"key": "rules/state-based-actions-combat", "version": "0.1.0"},
        {"key": "rules/turn-structure", "version": "0.1.0"},
        {"key": "rules/zone-incarnation", "version": "0.1.0"},
    ]:
        raise SystemExit("S3.A must use the exact reviewed turn/SBA/zone-incarnation closure")
    if basic_priority_entry["rules_authority"] != expected_authority or basic_priority_entry[
        "capability_closure"
    ] != [
        {"key": "rules/basic-priority", "version": "0.1.0"},
        {"key": "rules/state-based-actions-combat", "version": "0.1.0"},
        {"key": "rules/turn-structure", "version": "0.1.0"},
        {"key": "rules/zone-incarnation", "version": "0.1.0"},
    ]:
        raise SystemExit("S3.B must use the exact reviewed priority/SBA/turn/zone closure")
    if draw_entry["rules_authority"] != expected_authority or draw_entry["capability_closure"] != [
        {"key": "rules/basic-priority", "version": "0.1.0"},
        {"key": "rules/draw-card", "version": "0.1.0"},
        {"key": "rules/state-based-actions-combat", "version": "0.1.0"},
        {"key": "rules/turn-structure", "version": "0.1.0"},
        {"key": "rules/zone-incarnation", "version": "0.1.0"},
    ]:
        raise SystemExit("S3.C must use the exact reviewed draw/priority/SBA/turn/zone closure")
    if combat_entry["rules_authority"] != expected_authority or combat_entry[
        "capability_closure"
    ] != [
        {"key": "rules/basic-priority", "version": "0.1.0"},
        {"key": "rules/combat-phase", "version": "0.1.0"},
        {"key": "rules/declare-attackers", "version": "0.1.0"},
        {"key": "rules/draw-card", "version": "0.1.0"},
        {"key": "rules/state-based-actions-combat", "version": "0.1.0"},
        {"key": "rules/turn-structure", "version": "0.1.0"},
        {"key": "rules/zone-incarnation", "version": "0.1.0"},
    ]:
        raise SystemExit("combat profile must use the exact reviewed cumulative combat closure")
    if combat_blockers_entry["rules_authority"] != expected_authority or combat_blockers_entry[
        "capability_closure"
    ] != [
        {"key": "rules/basic-priority", "version": "0.1.0"},
        {"key": "rules/combat-phase", "version": "0.1.0"},
        {"key": "rules/declare-attackers", "version": "0.1.0"},
        {"key": "rules/declare-blockers", "version": "0.1.0"},
        {"key": "rules/draw-card", "version": "0.1.0"},
        {"key": "rules/state-based-actions-combat", "version": "0.1.0"},
        {"key": "rules/turn-structure", "version": "0.1.0"},
        {"key": "rules/zone-incarnation", "version": "0.1.0"},
    ]:
        raise SystemExit(
            "combat+blockers profile must use the exact reviewed cumulative blocker closure"
        )
    expected_damage_authority = {
        "variant": "comprehensive_rules",
        "snapshot_id": (
            "wotc-cr-2026-09-25-txt-20260925-sha256-"
            "8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca"
        ),
    }
    if combat_damage_entry["rules_authority"] != expected_damage_authority or combat_damage_entry[
        "capability_closure"
    ] != [
        {"key": "rules/basic-priority", "version": "0.1.0"},
        {"key": "rules/combat-damage", "version": "0.1.0"},
        {"key": "rules/combat-phase", "version": "0.1.0"},
        {"key": "rules/damage-and-life", "version": "0.1.0"},
        {"key": "rules/declare-attackers", "version": "0.1.0"},
        {"key": "rules/declare-blockers", "version": "0.1.0"},
        {"key": "rules/draw-card", "version": "0.1.0"},
        {"key": "rules/state-based-actions-combat", "version": "0.1.0"},
        {"key": "rules/turn-structure", "version": "0.1.0"},
        {"key": "rules/zone-incarnation", "version": "0.1.0"},
    ]:
        raise SystemExit("combat+damage profile must use the reviewed cumulative damage closure")
    if bounded_turn_entry["rules_authority"] != expected_damage_authority or bounded_turn_entry[
        "capability_closure"
    ] != [
        {"key": "rules/basic-priority", "version": "0.1.0"},
        {"key": "rules/cleanup-reset", "version": "0.1.0"},
        {"key": "rules/combat-damage", "version": "0.1.0"},
        {"key": "rules/combat-phase", "version": "0.1.0"},
        {"key": "rules/damage-and-life", "version": "0.1.0"},
        {"key": "rules/declare-attackers", "version": "0.1.0"},
        {"key": "rules/declare-blockers", "version": "0.1.0"},
        {"key": "rules/draw-card", "version": "0.1.0"},
        {"key": "rules/state-based-actions-combat", "version": "0.1.0"},
        {"key": "rules/turn-structure", "version": "0.1.0"},
        {"key": "rules/zone-incarnation", "version": "0.1.0"},
    ]:
        raise SystemExit("bounded turn profile must use the exact Foundation V2 closure")
    for entry in (
        turn_structure_entry,
        ordered_sba_entry,
        basic_priority_entry,
        draw_entry,
        combat_entry,
        combat_blockers_entry,
        combat_damage_entry,
        bounded_turn_entry,
    ):
        if entry["format_contract_id"] is not None or entry["content_contract_id"] is not None:
            raise SystemExit(
                "current S1/S3.A production identities have null format/content dimensions"
            )


def generated_bytes(content: str) -> bytes:
    return content.encode("utf-8")


def write_generated(target: Path, content: str) -> None:
    target.write_bytes(generated_bytes(content))


def generated_bytes_match(target: Path, content: str) -> bool:
    return target.is_file() and target.read_bytes() == generated_bytes(content)


def check_catalog_paths(targets: list[Path], catalog: dict[str, object] | None = None) -> int:
    """Policy-free comparison helper: renders the catalog target from the
    given catalog (or the current SOURCE_PATH) and reports drift.

    This is policy-free: it accepts any valid manifest and does NOT
    require both exact production S1 and S3.A Magic entries.
    """
    content = render_catalog_generated(catalog)
    drift = [str(target) for target in targets if not generated_bytes_match(target, content)]
    if drift:
        print("generated semantic-contract catalog drift:")
        for rel in drift:
            print(f"  - {rel}")
        return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    catalog = load_source()
    assert_production_policy(catalog)
    content = render_catalog_generated(catalog)
    if args.check:
        if check_catalog_paths([TARGET_PATH], catalog=catalog):
            return 1
        print("PASS: semantic-contract catalog matches source")
        return 0
    TARGET_PATH.parent.mkdir(parents=True, exist_ok=True)
    write_generated(TARGET_PATH, content)
    print(f"PASS: semantic-contract catalog generated to {TARGET_REL}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
