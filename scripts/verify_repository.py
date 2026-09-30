#!/usr/bin/env python3
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import ast
import json
import re
import subprocess
import tomllib
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[1]
SCAN_EXCLUDED_PARTS = {".git", ".venv", "target", "dist"}
IGNORED_PARTS = SCAN_EXCLUDED_PARTS | {
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
}
ACTION_USE_RE = re.compile(r"^\s*(?:-\s*)?uses:\s*(\S+)")
ACTION_PIN_RE = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+@[0-9a-f]{40}$")


def source_paths(pattern: str):
    return (
        path
        for path in ROOT.rglob(pattern)
        if not any(part in IGNORED_PARTS for part in path.relative_to(ROOT).parts)
    )


def fail(message: str) -> None:
    raise SystemExit(f"FAIL: {message}")


def action_pin_error(reference: str) -> str | None:
    if reference.startswith(("./", "../")):
        return None
    if ACTION_PIN_RE.fullmatch(reference):
        return None
    return "external GitHub Action must use a 40-character lowercase commit SHA"


def workflow_action_references() -> list[tuple[str, int, str]]:
    references: list[tuple[str, int, str]] = []
    workflow_root = ROOT / ".github" / "workflows"
    for workflow in sorted((*workflow_root.glob("*.yml"), *workflow_root.glob("*.yaml"))):
        relative = str(workflow.relative_to(ROOT))
        for number, line in enumerate(workflow.read_text(encoding="utf-8").splitlines(), start=1):
            match = ACTION_USE_RE.match(line)
            if match is not None:
                references.append((relative, number, match.group(1).strip("'\"")))
    return references


def workflow_action_pin_errors() -> list[str]:
    errors: list[str] = []
    for path, line, reference in workflow_action_references():
        problem = action_pin_error(reference)
        if problem is not None:
            errors.append(f"{path}:{line}: {problem}: {reference}")
    return errors


def is_bytecode(path: Path) -> bool:
    return "__pycache__" in path.parts or path.suffix in {".pyc", ".pyo"}


def local_bytecode() -> set[Path]:
    return {
        path.relative_to(ROOT)
        for path in ROOT.rglob("*")
        if not any(part in SCAN_EXCLUDED_PARTS for part in path.relative_to(ROOT).parts)
        and is_bytecode(path.relative_to(ROOT))
    }


def committable_bytecode() -> list[Path]:
    """Bytecode that could reach a commit or the source archive.

    In a git checkout, git-ignored caches (``**/__pycache__/``) are local
    output that neither git nor build_source_archive.py picks up, so only
    tracked or unignored bytecode counts. Without git, scan the whole tree.
    """
    if (ROOT / ".git").exists():
        listed = subprocess.run(
            ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
            cwd=ROOT,
            capture_output=True,
            check=True,
        ).stdout.decode("utf-8")
        return [path for path in map(Path, filter(None, listed.split("\0"))) if is_bytecode(path)]
    return sorted(local_bytecode())


def main() -> None:
    bytecode_before = local_bytecode()
    forbidden = committable_bytecode()
    if forbidden:
        fail(f"generated Python bytecode is present: {forbidden[:5]}")

    required = [
        "Cargo.toml",
        "rust-toolchain.toml",
        "python/pyproject.toml",
        "config/reproducibility.toml",
        "crates/mtgml-wire/src/lib.rs",
        "crates/mtgml-state/src/lib.rs",
        "crates/mtgml-environment/src/lib.rs",
        "schemas/episode-status.v1.schema.json",
        "schemas/capability-registry.v1.schema.json",
        "schemas/card-definition-manifest.v1.schema.json",
        "schemas/bundle-manifest.v1.schema.json",
        "schemas/bundle-certification.v1.schema.json",
        "schemas/normative-document-register.v1.schema.json",
        "wire/golden/manifest.json",
        "wire/negative/manifest.json",
        "docs/M0_2_SPECIFICATION.md",
        "docs/V0_2_1_CONTRACT_CLOSURE.md",
        "docs/NORMATIVE_HIERARCHY.md",
        "docs/DOMAIN_MODEL.md",
        "docs/EXECUTION_MODEL.md",
        "docs/cards/ADDING_CARDS.md",
        "docs/cards/CAPABILITY_MODEL.md",
        "docs/rules/ADDING_RULES_AND_MECHANICS.md",
        "docs/contracts/M0_2_DESIGN_LOCK_MATRIX.md",
        "docs/normative-document-register.v1.json",
        "cards/capabilities/registry.json",
        "scripts/capability_census.py",
        "scripts/certify_bundle.py",
        "scripts/check_documentation.py",
        "scripts/run_dependency_audit.py",
        "scripts/validate_maintainer_artifacts.py",
        "scripts/build_source_archive.py",
        "scripts/verify_source_archive.py",
        "scripts/verify_archive_reproducibility.py",
        "scripts/check_rust_source_structure.py",
        "contracts/catalog/contract-vocabulary.v1.json",
        "crates/mtgml-model/src/generated_contract_vocab.rs",
        "python/src/mtgml/_generated_contract_vocab.py",
        "docs/generated/CONTRACT_VOCABULARY.md",
        "docs/V0_2_2_EXECUTABLE_FREEZE_AND_MAINTAINER_ERGONOMICS.md",
        "docs/maintenance/MAINTAINER_PROFILES.md",
        "scripts/generate_contracts.py",
        "scripts/run_checks.py",
        "scripts/failure_packet.py",
        "scripts/capture_failure.py",
        "scripts/rerun_failure.py",
        "scripts/bootstrap.py",
        "scripts/validate_golden_path.py",
        "examples/golden-path/index.json",
        ".github/workflows/pr-fast.yml",
        ".github/workflows/integration.yml",
        ".github/workflows/nightly.yml",
        ".github/workflows/windows-setup-smoke.yml",
        ".github/workflows/dependency-audit.yml",
        "docs/maintenance/DEVELOPER_SETUP.md",
    ]
    missing = [path for path in required if not (ROOT / path).is_file()]
    if missing:
        fail(f"required files are missing: {missing}")

    action_pin_errors = workflow_action_pin_errors()
    if action_pin_errors:
        fail("invalid GitHub Action pins: " + "; ".join(action_pin_errors))

    if (ROOT / "crates/mtgml-engine-state").exists():
        fail("duplicate/orphan mtgml-engine-state crate is forbidden; mtgml-state is canonical")

    if (ROOT / "scripts/build_release_archive.py").exists():
        fail("duplicate archive builder is forbidden; build_source_archive.py is canonical")

    generated_source_reports = [
        "verification-results.json",
        "FOUNDATION_VERIFICATION.md",
        "FOUNDATION_BLOCKERS.md",
        "M0_2_STATUS.json",
        "M0_2_1_STATUS.json",
        "v0.2.2-status.json",
        "verification-logs",
    ]
    present_generated = [name for name in generated_source_reports if (ROOT / name).exists()]
    if present_generated:
        fail(
            f"generated verification output must remain outside source archive: {present_generated}"
        )

    with (ROOT / "config/reproducibility.toml").open("rb") as handle:
        reproducibility = tomllib.load(handle)
    if reproducibility != {
        "manifest_version": "reproducibility.v1",
        "source_date_epoch": 1787011200,
        "archive_prefix": "mtg-ml-engine-foundation-v0.2.2",
        "zip_compression": "deflate-9",
    }:
        fail("reproducibility configuration is incomplete or inconsistent")

    with (ROOT / "Cargo.toml").open("rb") as handle:
        cargo = tomllib.load(handle)
    workspace = cargo.get("workspace", {})
    members = workspace.get("members", [])
    if "crates/mtgml-wire" not in members or "crates/mtgml-state" not in members:
        fail("canonical wire/state crates are absent from the workspace")
    if workspace.get("package", {}).get("version") != "0.2.2":
        fail("workspace version is not 0.2.2")
    with (ROOT / "rustfmt.toml").open("rb") as handle:
        rustfmt = tomllib.load(handle)
    if rustfmt.get("edition") != workspace.get("package", {}).get("edition"):
        fail("rustfmt edition must match the Cargo workspace edition")
    if len(members) != len(set(members)):
        fail("duplicate Cargo workspace member")
    for member in members:
        if not (ROOT / member / "Cargo.toml").is_file():
            fail(f"workspace member missing Cargo.toml: {member}")

    with (ROOT / "python/pyproject.toml").open("rb") as handle:
        python_project = tomllib.load(handle)
    if python_project.get("project", {}).get("version") != "0.2.2":
        fail("Python package version is not 0.2.2")

    for path in sorted(source_paths("*.json")):
        try:
            json.loads(path.read_text(encoding="utf-8"))
        except Exception as exc:
            fail(f"invalid JSON in {path.relative_to(ROOT)}: {exc}")
    for path in sorted(source_paths("*.toml")):
        try:
            with path.open("rb") as handle:
                tomllib.load(handle)
        except Exception as exc:
            fail(f"invalid TOML in {path.relative_to(ROOT)}: {exc}")
    for path in sorted(source_paths("*.py")):
        try:
            ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        except SyntaxError as exc:
            fail(f"invalid Python in {path.relative_to(ROOT)}: {exc}")

    for path in sorted(list(source_paths("*.yml")) + list(source_paths("*.yaml"))):
        try:
            yaml.safe_load(path.read_text(encoding="utf-8"))
        except Exception as exc:
            fail(f"invalid YAML in {path.relative_to(ROOT)}: {exc}")

    golden = json.loads((ROOT / "wire/golden/manifest.json").read_text(encoding="utf-8"))[
        "fixtures"
    ]
    negative = json.loads((ROOT / "wire/negative/manifest.json").read_text(encoding="utf-8"))[
        "fixtures"
    ]
    if len(golden) < 10 or len(negative) < 13:
        fail("shared fixture suites are incomplete")
    for directory, cases in (("golden", golden), ("negative", negative)):
        for case in cases:
            if not (ROOT / "wire" / directory / case["path"]).is_file():
                fail(f"missing {directory} fixture {case['path']}")
            if (
                directory == "negative"
                and case.get("expected_reject_layer") != "rust-python-semantic-or-decode"
            ):
                fail(f"negative fixture has unsupported reject layer: {case['path']}")

    catalog = json.loads(
        (ROOT / "contracts/catalog/contract-vocabulary.v1.json").read_text(encoding="utf-8")
    )
    catalog_codes = set(catalog["stable_wire_error_codes"])
    rust_wire_text = (ROOT / "crates/mtgml-wire/src/lib.rs").read_text(encoding="utf-8")
    python_wire_text = "\n".join(
        path.read_text(encoding="utf-8")
        for path in sorted((ROOT / "python/src/mtgml").glob("*.py"))
    )
    rust_codes = set(re.findall(r'WireError::new\("([^"]+)"', rust_wire_text))
    python_codes = set(re.findall(r'WireError\("([^"]+)"', python_wire_text))
    observed_codes = rust_codes | python_codes
    if observed_codes != catalog_codes:
        fail(
            "stable wire error-code catalog differs from source use: "
            f"missing={sorted(observed_codes - catalog_codes)}, "
            f"stale={sorted(catalog_codes - observed_codes)}"
        )

    justfile = (ROOT / "justfile").read_text(encoding="utf-8")
    for recipe in (
        "doctor:",
        "bootstrap:",
        "check-fast:",
        "check:",
        "check-all:",
        "audit-dependencies:",
        "release-candidate:",
    ):
        if recipe not in justfile:
            fail(f"maintainer justfile lacks {recipe}")
    if (ROOT / ".github/workflows/ci.yml").exists():
        fail("legacy monolithic CI workflow must not coexist with V0.2.2 split profiles")

    player_python = (ROOT / "python/src/mtgml/player_client.py").read_text(encoding="utf-8")
    for token in ("information_state", "PlayerStep", "visible_decision", "submit"):
        if token not in player_python:
            fail(f"Python PlayerClient lacks {token}")

    wire_rust = (ROOT / "crates/mtgml-wire/src/lib.rs").read_text(encoding="utf-8")
    if (
        "verify_negative_fixture_directory" not in wire_rust
        or "every_shared_negative_fixture" not in wire_rust
    ):
        fail("Rust shared negative fixture test is not implemented")

    env_src = ROOT / "crates/mtgml-environment/src"
    env_prod = [p for p in sorted(env_src.glob("*.rs")) if p.name != "tests.rs"]
    env_rust = "\n".join(p.read_text(encoding="utf-8") for p in env_prod)

    if "Arc<Mutex" not in env_rust or re.search(r"fn\s+bind_player\s*\(\s*&self", env_rust) is None:
        fail("player endpoint handles still borrow the controller exclusively")

    state_src = ROOT / "crates/mtgml-state/src"
    production_files = [p for p in sorted(state_src.glob("*.rs")) if p.name != "tests.rs"]
    state_rust = "\n".join(p.read_text(encoding="utf-8") for p in production_files)
    for token in (
        "validate_engine_state",
        "pub struct EngineState",
        "pub replacement:",
        "PerspectiveIdentityState",
    ):
        if token not in state_rust:
            fail(f"state contract lacks {token}")

    for token in (
        "full-state-digest-input.v7",
        "magic_sba_graveyard_order_v1",
        "canonical_state_bytes",
        "KnowledgeInvalidationReason",
        "KnowledgeAcquisitionReason",
    ):
        if token not in state_rust:
            fail(f"state contract closure lacks {token}")
    maintainer = (ROOT / "scripts/maintainer_common.py").read_text(encoding="utf-8")
    for token in (
        "discovered_native_executors",
        "undeclared_native_executors",
        "stale_native_executor_declarations",
    ):
        if token not in maintainer:
            fail(f"native executor closure lacks {token}")

    verification_runner = (ROOT / "scripts/run_verification.py").read_text(encoding="utf-8")
    if 'ROOT / "dist" / "verification"' not in verification_runner or verification_runner.rfind(
        "archive_reproducibility"
    ) < verification_runner.rfind("cargo_test"):
        fail("verification output is not external or archive gate is not last")
    for token in (
        "source_tree_fingerprint",
        '"source_tree_unchanged"',
        "OUTPUT_MARKER",
    ):
        if token not in verification_runner:
            fail(f"verification runner safety contract lacks {token}")

    sys.path.insert(0, str(ROOT / "python" / "src"))
    from mtgml.errors import WireError
    from mtgml.wire import decode_canonical, encode_canonical

    for case in golden:
        payload = (ROOT / "wire/golden" / case["path"]).read_bytes()
        decoded = decode_canonical(case["contract"], payload)
        if encode_canonical(decoded) != payload:
            fail(f"golden fixture is not roundtrip-closed: {case['path']}")
    for case in negative:
        payload = (ROOT / "wire/negative" / case["path"]).read_bytes()
        try:
            decode_canonical(case["contract"], payload)
        except WireError as error:
            if error.code != case["expected_error_code"]:
                fail(
                    f"negative fixture {case['path']} returned"
                    f" {error.code}, expected {case['expected_error_code']}"
                )
        else:
            fail(f"negative fixture was accepted: {case['path']}")

    created = sorted(local_bytecode() - bytecode_before)
    if created:
        fail(f"verifier created Python bytecode: {created[:5]}")

    file_count = sum(1 for path in source_paths("*") if path.is_file())
    print(
        f"PASS: V0.2.2 repository contracts verified"
        f" ({file_count} files, {len(golden)} golden, {len(negative)} negative fixtures)"
    )


if __name__ == "__main__":
    main()
