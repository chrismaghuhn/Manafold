#!/usr/bin/env python3
"""Generate the V5 semantic-contract catalog constants (spec §10 chain).

Single generator for the single hand-authored manifest source
``contracts/catalog/semantic-contracts.v1.json``. Emits BOTH the Rust manifest
constants AND the derived ``RulesContractIdV1``/``SemanticContractIdV1``
values into ``crates/mtgml-environment/src/semantic_catalog_generated.rs``.

Layering (review Fix-02):

- ``render_generated`` is a pure renderer of VALID manifest facts. It derives
  IDs exclusively via the Task-2 Python mechanical mirrors (no digest logic of
  its own) and can render any manifest the §7 contract accepts — including a
  hypothetical ComprehensiveRules entry in a scratch source (used by tests and
  by the later S1 slice that will extend the production source).
- ``assert_production_policy`` is the PRODUCTION source validator. The
  checked-in production catalog allows exactly one synthetic_legacy entry with
  null closure and null dimensions; a valid ComprehensiveRules production
  entry is refused here, not because it is invalid but by V5 catalog policy.
- The CLI (``main``) always enforces the production policy before rendering.

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


def render_generated(catalog: dict[str, object] | None = None) -> str:
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
    lines.append("#![allow(dead_code)] // consumed by the KAT now and the Task-8 runtime later")
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
        lines.append(
            f"pub fn {snake}_semantic_manifest() -> mtgml_model::SemanticContractManifestV1 {{"
        )
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
    """PRODUCTION source policy (V5 slice): exactly one synthetic_legacy
    entry with a null capability closure. Dimension nullity is deliberately
    NOT checked here — non-null format/content dimensions are refused at the
    emission boundary (``render_generated``), which the CLI always runs after
    this validator. This validator owns the entry-count/variant surface."""
    entries = catalog["entries"]
    if len(entries) != 1:
        raise SystemExit(
            f"production semantic-contract catalog must contain exactly one entry; "
            f"got {len(entries)}"
        )
    entry = entries[0]
    if not isinstance(entry, dict):
        raise SystemExit("catalog entry must be an object")
    validate_entry_facts(entry)
    authority = entry["rules_authority"]
    if not isinstance(authority, dict) or authority.get("variant") != "synthetic_legacy":
        raise SystemExit(
            "production semantic-contract catalog allows only synthetic_legacy entries"
        )
    if entry["capability_closure"] is not None:
        raise SystemExit("production synthetic entry must carry a null capability closure")


def generated_bytes(content: str) -> bytes:
    return content.encode("utf-8")


def write_generated(target: Path, content: str) -> None:
    target.write_bytes(generated_bytes(content))


def generated_bytes_match(target: Path, content: str) -> bool:
    return target.is_file() and target.read_bytes() == generated_bytes(content)


def check_paths(targets: list[Path], catalog: dict[str, object] | None = None) -> int:
    """Policy-free comparison helper: renders (from the given catalog, or the
    current SOURCE_PATH) and reports drift against the targets. The CLI adds
    the production policy on top."""
    content = render_generated(catalog)
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
    content = render_generated(catalog)
    if args.check:
        if not generated_bytes_match(TARGET_PATH, content):
            print(f"generated semantic-contract catalog drift: {TARGET_REL}")
            return 1
        print("PASS: semantic-contract catalog matches source")
        return 0
    TARGET_PATH.parent.mkdir(parents=True, exist_ok=True)
    write_generated(TARGET_PATH, content)
    print(f"PASS: semantic-contract catalog generated to {TARGET_REL}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
