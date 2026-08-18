#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
CAPABILITY_RE = re.compile(r"^(rules|mechanic|decision|visibility|tooling|format/[a-z0-9-]+)/[a-z0-9][a-z0-9-]*(/[a-z0-9][a-z0-9-]*)*$")
ARTIFACT_ID_RE = re.compile(r"^[a-z0-9][a-z0-9-]*(/[a-z0-9][a-z0-9-]*)+$")
LIFECYCLE_ORDER = {
    "proposed": 0,
    "specified": 1,
    "implemented": 2,
    "covered": 3,
    "certified": 4,
    "deprecated": -1,
}
CARD_LIFECYCLE_ORDER = {
    "draft": 0,
    "imported": 1,
    "parsed": 2,
    "implemented": 3,
    "covered": 4,
    "certified": 5,
    "blocked": -1,
    "deprecated": -1,
}
CAPABILITY_LIFECYCLE_THRESHOLDS = tuple(
    key for key, order in LIFECYCLE_ORDER.items() if order >= 0
)
CARD_LIFECYCLE_THRESHOLDS = tuple(
    key for key, order in CARD_LIFECYCLE_ORDER.items() if order >= 0
)


class MaintainerArtifactError(ValueError):
    pass


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise MaintainerArtifactError(f"cannot read JSON {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise MaintainerArtifactError(f"expected JSON object in {path}")
    return value


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def canonical_json_bytes(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True) + "\n").encode("utf-8")


def digest_json(value: object) -> str:
    return hashlib.sha256(canonical_json_bytes(value)).hexdigest()


def ensure_artifact_id(value: str, label: str) -> None:
    if ARTIFACT_ID_RE.fullmatch(value) is None:
        raise MaintainerArtifactError(f"invalid {label}: {value!r}")


def ensure_capability_key(value: str) -> None:
    if CAPABILITY_RE.fullmatch(value) is None:
        raise MaintainerArtifactError(f"invalid capability key: {value!r}")


def resolve_repository_path(root: Path, value: str, label: str) -> Path:
    relative = Path(value)
    if relative.is_absolute() or ".." in relative.parts or not relative.parts:
        raise MaintainerArtifactError(f"unsafe {label}: {value!r}")
    resolved = (root / relative).resolve()
    try:
        resolved.relative_to(root.resolve())
    except ValueError as exc:
        raise MaintainerArtifactError(f"{label} escapes repository: {value!r}") from exc
    return resolved


def definition_directory(definition_id: str, root: Path = ROOT) -> Path:
    ensure_artifact_id(definition_id, "definition_id")
    return root / "cards" / "definitions" / Path(*definition_id.split("/"))


def find_definition_manifest(definition_id: str, root: Path = ROOT) -> Path | None:
    directory = definition_directory(definition_id, root)
    for name in ("manifest.json", "manifest.example.json"):
        candidate = directory / name
        if candidate.is_file():
            return candidate
    return None


def validate_capability_registry(
    registry: dict[str, Any],
    *,
    root: Path | None = None,
) -> tuple[dict[str, dict[str, Any]], list[list[str]]]:
    if registry.get("schema_version") != "capability-registry.v1":
        raise MaintainerArtifactError("unsupported capability registry version")
    registry_id = registry.get("registry_id")
    if not isinstance(registry_id, str):
        raise MaintainerArtifactError("registry_id must be a string")
    ensure_artifact_id(registry_id, "registry_id")
    entries = registry.get("entries")
    if not isinstance(entries, list):
        raise MaintainerArtifactError("registry entries must be a list")

    by_key: dict[str, dict[str, Any]] = {}
    for raw in entries:
        if not isinstance(raw, dict):
            raise MaintainerArtifactError("capability entry must be an object")
        key = raw.get("key")
        if not isinstance(key, str):
            raise MaintainerArtifactError("capability key must be a string")
        ensure_capability_key(key)
        if key in by_key:
            raise MaintainerArtifactError(f"duplicate capability key: {key}")
        lifecycle = raw.get("lifecycle")
        if lifecycle not in LIFECYCLE_ORDER:
            raise MaintainerArtifactError(f"invalid lifecycle for {key}: {lifecycle!r}")
        if root is not None:
            spec_value = raw.get("spec_path")
            if not isinstance(spec_value, str):
                raise MaintainerArtifactError(f"spec_path must be a string for {key}")
            spec_path = resolve_repository_path(root, spec_value, f"spec_path for {key}")
            if not spec_path.is_file():
                raise MaintainerArtifactError(f"missing capability specification for {key}: {spec_value}")

            implementation_paths = raw.get("implementation_paths", [])
            if not isinstance(implementation_paths, list) or any(
                not isinstance(item, str) for item in implementation_paths
            ):
                raise MaintainerArtifactError(f"invalid implementation_paths for {key}")
            if LIFECYCLE_ORDER[lifecycle] >= LIFECYCLE_ORDER["implemented"] and not implementation_paths:
                raise MaintainerArtifactError(
                    f"{key}@{lifecycle} requires at least one implementation path"
                )
            for implementation_value in implementation_paths:
                implementation_path = resolve_repository_path(
                    root,
                    implementation_value,
                    f"implementation path for {key}",
                )
                if not implementation_path.exists():
                    raise MaintainerArtifactError(
                        f"missing implementation path for {key}: {implementation_value}"
                    )

            authority_refs = raw.get("authority_refs", [])
            if LIFECYCLE_ORDER[lifecycle] >= LIFECYCLE_ORDER["specified"] and not authority_refs:
                raise MaintainerArtifactError(f"{key}@{lifecycle} requires authority references")
            if (
                LIFECYCLE_ORDER[lifecycle] >= LIFECYCLE_ORDER["specified"]
                and raw.get("information_risk") == "unreviewed"
            ):
                raise MaintainerArtifactError(f"{key}@{lifecycle} has unreviewed information risk")
            if (
                LIFECYCLE_ORDER[lifecycle] >= LIFECYCLE_ORDER["covered"]
                and not raw.get("conformance_cases")
            ):
                raise MaintainerArtifactError(f"{key}@{lifecycle} requires conformance cases")
        by_key[key] = raw

    for key, entry in by_key.items():
        dependencies = entry.get("dependencies", [])
        if not isinstance(dependencies, list) or any(not isinstance(dep, str) for dep in dependencies):
            raise MaintainerArtifactError(f"invalid dependencies for {key}")
        if len(dependencies) != len(set(dependencies)):
            raise MaintainerArtifactError(f"duplicate dependency for {key}")
        for dep in dependencies:
            ensure_capability_key(dep)
            if dep not in by_key:
                raise MaintainerArtifactError(f"unregistered dependency {dep} required by {key}")

    cycles = find_dependency_cycles(by_key)
    return by_key, cycles


def find_dependency_cycles(by_key: dict[str, dict[str, Any]]) -> list[list[str]]:
    cycles: list[list[str]] = []
    visiting: list[str] = []
    state: dict[str, int] = {}

    def visit(key: str) -> None:
        marker = state.get(key, 0)
        if marker == 2:
            return
        if marker == 1:
            index = visiting.index(key)
            cycle = visiting[index:] + [key]
            if cycle not in cycles:
                cycles.append(cycle)
            return
        state[key] = 1
        visiting.append(key)
        for dep in by_key[key].get("dependencies", []):
            visit(dep)
        visiting.pop()
        state[key] = 2

    for key in sorted(by_key):
        visit(key)
    return cycles


def validate_card_manifest(
    manifest: dict[str, Any],
    *,
    root: Path | None = None,
) -> None:
    if manifest.get("schema_version") != "card-definition-manifest.v1":
        raise MaintainerArtifactError("unsupported card definition manifest version")
    definition_id = manifest.get("definition_id")
    if not isinstance(definition_id, str):
        raise MaintainerArtifactError("definition_id must be a string")
    ensure_artifact_id(definition_id, "definition_id")
    capabilities = manifest.get("required_capabilities", [])
    if not isinstance(capabilities, list) or any(not isinstance(key, str) for key in capabilities):
        raise MaintainerArtifactError(f"invalid required capabilities for {definition_id}")
    if len(capabilities) != len(set(capabilities)):
        raise MaintainerArtifactError(f"duplicate required capability for {definition_id}")
    for key in capabilities:
        ensure_capability_key(key)
    lifecycle = manifest.get("lifecycle")
    if lifecycle not in CARD_LIFECYCLE_ORDER:
        raise MaintainerArtifactError(
            f"invalid card lifecycle for {definition_id}: {lifecycle!r}"
        )
    generated = manifest.get("generated_objects", [])
    if not isinstance(generated, list) or any(not isinstance(item, str) for item in generated):
        raise MaintainerArtifactError(f"invalid generated objects for {definition_id}")
    if len(generated) != len(set(generated)):
        raise MaintainerArtifactError(f"duplicate generated object for {definition_id}")
    for item in generated:
        ensure_artifact_id(item, "generated definition ID")
    native_executor = manifest.get("native_executor")
    if native_executor is not None and (
        not isinstance(native_executor, str) or not native_executor
    ):
        raise MaintainerArtifactError(
            f"invalid native_executor for {definition_id}"
        )
    if root is not None:
        implementation_value = manifest.get("implementation_path")
        if CARD_LIFECYCLE_ORDER[lifecycle] >= CARD_LIFECYCLE_ORDER["implemented"]:
            if not isinstance(implementation_value, str):
                raise MaintainerArtifactError(
                    f"{definition_id}@{lifecycle} requires implementation_path"
                )
            implementation_path = resolve_repository_path(
                root,
                implementation_value,
                f"implementation_path for {definition_id}",
            )
            if not implementation_path.exists():
                raise MaintainerArtifactError(
                    f"missing card implementation for {definition_id}: {implementation_value}"
                )
        if (
            CARD_LIFECYCLE_ORDER[lifecycle] >= CARD_LIFECYCLE_ORDER["covered"]
            and not manifest.get("conformance_cases")
        ):
            raise MaintainerArtifactError(
                f"{definition_id}@{lifecycle} requires conformance cases"
            )


def validate_bundle_manifest(bundle: dict[str, Any]) -> None:
    if bundle.get("schema_version") != "bundle-manifest.v1":
        raise MaintainerArtifactError("unsupported bundle manifest version")
    bundle_id = bundle.get("bundle_id")
    if not isinstance(bundle_id, str):
        raise MaintainerArtifactError("bundle_id must be a string")
    ensure_artifact_id(bundle_id, "bundle_id")
    players: list[int] = []
    for deck in bundle.get("decks", []):
        if not isinstance(deck, dict) or not isinstance(deck.get("player"), int):
            raise MaintainerArtifactError("invalid deck identity")
        players.append(deck["player"])
    if len(players) != len(set(players)):
        raise MaintainerArtifactError("duplicate player in deck identities")
    for field in ("definition_ids", "generated_definition_ids"):
        values = bundle.get(field, [])
        if not isinstance(values, list) or any(not isinstance(value, str) for value in values):
            raise MaintainerArtifactError(f"invalid {field}")
        if len(values) != len(set(values)):
            raise MaintainerArtifactError(f"duplicate value in {field}")
        for value in values:
            ensure_artifact_id(value, field)
    definition_ids = set(bundle.get("definition_ids", []))
    generated_definition_ids = set(bundle.get("generated_definition_ids", []))
    overlap = sorted(definition_ids & generated_definition_ids)
    if overlap:
        raise MaintainerArtifactError(
            f"definition_ids and generated_definition_ids overlap: {overlap}"
        )
    capabilities = bundle.get("required_capabilities", [])
    if not isinstance(capabilities, list) or any(not isinstance(key, str) for key in capabilities):
        raise MaintainerArtifactError("invalid required_capabilities")
    if len(capabilities) != len(set(capabilities)):
        raise MaintainerArtifactError("duplicate required capability")
    for key in capabilities:
        ensure_capability_key(key)
    native_executors = bundle.get("native_executors", [])
    if not isinstance(native_executors, list) or any(
        not isinstance(item, str) or not item for item in native_executors
    ):
        raise MaintainerArtifactError("invalid native_executors")
    if len(native_executors) != len(set(native_executors)):
        raise MaintainerArtifactError("duplicate native executor")


@dataclass(frozen=True)
class CapabilityCensus:
    required: tuple[str, ...]
    resolved: tuple[str, ...]
    missing: tuple[str, ...]
    cycles: tuple[tuple[str, ...], ...]
    below_required_lifecycle: tuple[tuple[str, str], ...]
    missing_definitions: tuple[str, ...]
    card_lifecycle_blockers: tuple[tuple[str, str], ...]
    native_executors: tuple[str, ...]
    undeclared_native_executors: tuple[str, ...]
    stale_native_executor_declarations: tuple[str, ...]

    def to_dict(self) -> dict[str, object]:
        return {
            "required": list(self.required),
            "resolved": list(self.resolved),
            "missing": list(self.missing),
            "cycles": [list(path) for path in self.cycles],
            "below_required_lifecycle": [
                {"key": key, "lifecycle": lifecycle}
                for key, lifecycle in self.below_required_lifecycle
            ],
            "missing_definitions": list(self.missing_definitions),
            "card_lifecycle_blockers": [
                {"definition_id": definition_id, "lifecycle": lifecycle}
                for definition_id, lifecycle in self.card_lifecycle_blockers
            ],
            "native_executors": list(self.native_executors),
            "undeclared_native_executors": list(self.undeclared_native_executors),
            "stale_native_executor_declarations": list(
                self.stale_native_executor_declarations
            ),
        }


def capability_census(
    bundle: dict[str, Any],
    registry: dict[str, Any],
    *,
    root: Path = ROOT,
    required_capability_lifecycle: str = "specified",
    required_card_lifecycle: str = "draft",
) -> CapabilityCensus:
    validate_bundle_manifest(bundle)
    if required_capability_lifecycle not in CAPABILITY_LIFECYCLE_THRESHOLDS:
        raise MaintainerArtifactError(
            f"unknown required capability lifecycle: {required_capability_lifecycle!r}"
        )
    if required_card_lifecycle not in CARD_LIFECYCLE_THRESHOLDS:
        raise MaintainerArtifactError(
            f"unknown required card lifecycle: {required_card_lifecycle!r}"
        )
    by_key, cycle_paths = validate_capability_registry(registry, root=root)

    required: set[str] = set(bundle.get("required_capabilities", []))
    missing_definitions: list[str] = []
    card_blockers: list[tuple[str, str]] = []
    definition_ids = list(bundle.get("definition_ids", [])) + list(bundle.get("generated_definition_ids", []))
    visited_definitions: set[str] = set()
    discovered_native_executors: set[str] = set()

    queue = list(definition_ids)
    while queue:
        definition_id = queue.pop(0)
        if definition_id in visited_definitions:
            continue
        visited_definitions.add(definition_id)
        path = find_definition_manifest(definition_id, root)
        if path is None:
            missing_definitions.append(definition_id)
            continue
        manifest = load_json(path)
        validate_card_manifest(manifest, root=root)
        if manifest["definition_id"] != definition_id:
            raise MaintainerArtifactError(f"definition path/id mismatch: {definition_id} vs {manifest['definition_id']}")
        required.update(manifest.get("required_capabilities", []))
        queue.extend(manifest.get("generated_objects", []))
        native_executor = manifest.get("native_executor")
        if native_executor is not None:
            if not isinstance(native_executor, str) or not native_executor:
                raise MaintainerArtifactError(
                    f"invalid native_executor for {definition_id}"
                )
            discovered_native_executors.add(native_executor)
        lifecycle = manifest.get("lifecycle", "draft")
        if CARD_LIFECYCLE_ORDER.get(lifecycle, -1) < CARD_LIFECYCLE_ORDER[required_card_lifecycle]:
            card_blockers.append((definition_id, lifecycle))

    resolved: set[str] = set()
    missing: set[str] = set()
    stack = list(required)
    while stack:
        key = stack.pop()
        if key in resolved or key in missing:
            continue
        entry = by_key.get(key)
        if entry is None:
            missing.add(key)
            continue
        resolved.add(key)
        stack.extend(entry.get("dependencies", []))

    minimum = LIFECYCLE_ORDER[required_capability_lifecycle]
    below = sorted(
        (key, str(by_key[key].get("lifecycle", "unknown")))
        for key in resolved
        if LIFECYCLE_ORDER.get(by_key[key].get("lifecycle", ""), -1) < minimum
    )
    cycles = sorted(tuple(path) for path in cycle_paths)
    declared_native_executors = set(bundle.get("native_executors", []))
    undeclared_native_executors = discovered_native_executors - declared_native_executors
    stale_native_executor_declarations = declared_native_executors - discovered_native_executors
    return CapabilityCensus(
        required=tuple(sorted(required)),
        resolved=tuple(sorted(resolved)),
        missing=tuple(sorted(missing)),
        cycles=tuple(cycles),
        below_required_lifecycle=tuple(below),
        missing_definitions=tuple(sorted(set(missing_definitions))),
        card_lifecycle_blockers=tuple(sorted(set(card_blockers))),
        native_executors=tuple(sorted(discovered_native_executors)),
        undeclared_native_executors=tuple(sorted(undeclared_native_executors)),
        stale_native_executor_declarations=tuple(
            sorted(stale_native_executor_declarations)
        ),
    )


def certification_closure(census: CapabilityCensus) -> dict[str, object]:
    closure = census.to_dict()
    return {
        "required": closure["required"],
        "resolved": closure["resolved"],
        "missing": closure["missing"],
        "cycles": closure["cycles"],
        "below_certified": closure["below_required_lifecycle"],
        "missing_definitions": closure["missing_definitions"],
        "card_lifecycle_blockers": closure["card_lifecycle_blockers"],
        "native_executors": closure["native_executors"],
        "undeclared_native_executors": closure["undeclared_native_executors"],
        "stale_native_executor_declarations": closure[
            "stale_native_executor_declarations"
        ],
    }


def certification_status(census: CapabilityCensus, gates: Iterable[dict[str, Any]]) -> str:
    gate_list = list(gates)
    blocked = (
        census.missing
        or census.cycles
        or census.below_required_lifecycle
        or census.missing_definitions
        or census.card_lifecycle_blockers
        or census.native_executors
        or census.undeclared_native_executors
        or census.stale_native_executor_declarations
        or not gate_list
        or any(gate.get("status") != "PASS" for gate in gate_list)
    )
    return "blocked" if blocked else "certified"
