#!/usr/bin/env python3
"""Fail-closed MASTER_DRIFT closure checker for the imported M2.5 REV3 baseline.

The closure record sources/m2_5/pre_research/REV3/master_drift_closure_REV3.json
grants MASTER_DRIFT = PASS for one verified master SHA. This script proves that
grant still holds for the repository state under test and refuses silently
passing stale, mismatched, or tampered identities.

PASS requires all of:
  - the closure record exists, parses, and carries the expected schema;
  - the closure grants MASTER_DRIFT = PASS;
  - the closure and IMPORT_PROVENANCE.json agree exactly about the verified
    master SHA, the REV3 baseline repository SHA, the source package digest,
    and the MASTER_DRIFT = PASS grant itself (a syntactically valid but
    substituted SHA therefore fails);
  - the verified SHA is a syntactically valid git object id and the REV3
    baseline SHA is a git ancestor of it;
  - the original verified SHA is first validated through the historical
    descendant range to the previous effective passing head;
  - the additive revalidation advances that effective anchor to the reviewed
    post-#83 master, after which ordinary descendant drift checking applies;
  - every promoted evidence file is digest-covered either by
    IMPORT_PROVENANCE.json or by the closure's own bound_records section and
    still matches its recorded SHA-256; only the closure record itself is
    exempt (it is the root of trust anchored by reviewed git history).

Anything else exits non-zero with a precise diagnostic (FAIL) or, when evidence
cannot be evaluated at all, exits BLOCKED.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from collections.abc import Callable
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_PROVENANCE_DIR = ROOT / "sources" / "m2_5" / "pre_research" / "REV3"
CLOSURE_FILENAME = "master_drift_closure_REV3.json"
PROVENANCE_FILENAME = "IMPORT_PROVENANCE.json"
REPORT_FILENAME = "MASTER_DRIFT_REPORT.md"
REVALIDATION_FILENAME = "master_drift_revalidation_1a1e504.v1.json"
REVALIDATION_REPORT_FILENAME = "MASTER_DRIFT_REVALIDATION_1A1E504.md"
EXPECTED_CLOSURE_SCHEMA = "manafold.m2.5.a.master-drift-closure.v1"
EXPECTED_PROVENANCE_SCHEMA = "manafold.m2.5.a.import-provenance.v1"
EXPECTED_REVALIDATION_SCHEMA = "manafold.m2.5.a.master-drift-revalidation.v1"
EXPECTED_PREVIOUS_EFFECTIVE_HEAD = "df3d760de2c6b22403764725e0ef707161bbce13"
EXPECTED_NEW_REVALIDATED_MASTER = "1a1e504cf2e6232d5b8da47bdfb989980aa41884"
EXPECTED_REPAIR_HEAD = "34e23c57203b775d43e06e7946766566e4002a99"
EXPECTED_REPAIR_PARENT = "b9765aa45321cb36a2a6531aa613bcb2788b1d26"
EXPECTED_REVIEWED_COMMITS = [
    EXPECTED_REPAIR_PARENT,
    EXPECTED_REPAIR_HEAD,
    EXPECTED_NEW_REVALIDATED_MASTER,
]
EXPECTED_REVIEWED_PATHS = [
    "pytest.ini",
    "python/tests/test_m2_h_gate_runner.py",
    "scripts/run_python_tests.py",
]
REVALIDATION_TOP_LEVEL_KEYS = frozenset(
    {
        "schema",
        "revalidation_id",
        "historical_identity",
        "reviewed_range",
        "normative_drift_review",
        "semantic_impact_review",
        "outcome",
        "bound_evidence",
    }
)
HISTORICAL_IDENTITY_KEYS = frozenset(
    {
        "original_rev3_recorded_repository_sha",
        "original_import_verified_master_sha",
        "original_closure",
        "original_import_provenance",
        "original_report",
        "rev3_package",
    }
)
REVIEWED_RANGE_KEYS = frozenset(
    {
        "previous_effective_passing_head",
        "new_revalidated_master",
        "merge_commit",
        "merge_parents",
        "repair_head",
        "repair_parent",
        "reviewed_commit_range",
        "reviewed_commit_shas",
        "changed_paths",
    }
)
NORMATIVE_REVIEW_KEYS = frozenset({"control_paths", "unchanged_paths"})
SEMANTIC_IMPACT_KEYS = frozenset(
    {"material_semantic_drift", "impact_classification", "surface_results"}
)
SEMANTIC_SURFACE_KEYS = frozenset(
    {
        "magic_rules",
        "authoritative_rust_game_semantics",
        "state",
        "rng",
        "decisions",
        "observations",
        "information_state",
        "replay_checkpoints",
        "card_ir",
        "capability_definitions",
        "b1_authority",
        "b1_final_authority",
        "b2_classification_family_semantics",
        "rev3_candidate_universe",
        "interaction_model_input",
        "ranking_formula_input",
        "deck_pair_status",
        "m3_status",
        "python_test_infrastructure",
    }
)
OUTCOME_KEYS = frozenset(
    {
        "MASTER_DRIFT",
        "rev3_evidence_reusable",
        "research_data_revalidation_required",
        "effective_descendant_anchor",
    }
)
BOUND_EVIDENCE_KEYS = frozenset({"revalidation_report"})
BOUND_FILE_KEYS = frozenset({"path", "raw_sha256"})
ADDITIVE_REVALIDATION_FILES = frozenset({REVALIDATION_FILENAME, REVALIDATION_REPORT_FILENAME})
GIT_SHA_RE = re.compile(r"^[0-9a-f]{40}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
ALLOWED_EXACT_PATHS = frozenset(
    {
        "scripts/check_m2_5_master_drift.py",
        "scripts/check_m2_5_b1_authority_citations.py",
        "scripts/check_m2_5_b1_final_authority_citations.py",
        "scripts/check_m2_5_b2_classifications.py",
    }
)
ALLOWED_DIRECTORY_PREFIXES = (
    "sources/m2_5/pre_research/REV3/",
    "sources/m2_5/closures/B1/",
    "sources/m2_5/closures/B2/",
)
NORMATIVE_DRIFT_CONTROL_PATHS = (
    "crates/mtgml-rules/src/lib.rs",
    "python/src/mtgml/observation.py",
    "schemas/player-step.v2.schema.json",
    "wire/golden/manifest.json",
    "docs/contracts/WIRE_CONTRACT.md",
    "docs/adr/0041-capability-oriented-semantic-domains-and-explicit-semantic-ownership.md",
    "cards/capabilities/registry.json",
)
ARCHIVE_ENV_VAR = "MANAFOLD_SOURCE_ARCHIVE"

EXIT_PASS = 0
EXIT_FAIL = 1
EXIT_BLOCKED = 2


class DriftCheckError(Exception):
    def __init__(self, status: str, message: str, code: str | None = None) -> None:
        super().__init__(message)
        self.status = status
        self.message = message
        self.code = code


def path_is_allowed(path: str) -> bool:
    normalized = path.replace("\\", "/")
    return normalized in ALLOWED_EXACT_PATHS or normalized.startswith(ALLOWED_DIRECTORY_PREFIXES)


def read_json(path: Path) -> object:
    if not path.is_file():
        raise DriftCheckError("BLOCKED", f"missing required evidence file: {path}")
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise DriftCheckError("BLOCKED", f"unreadable JSON evidence {path}: {exc}") from exc


def load_revalidation(provenance_dir: Path) -> dict[str, object]:
    return require_mapping(read_json(provenance_dir / REVALIDATION_FILENAME), REVALIDATION_FILENAME)


def clone_mapping(value: dict[str, object], label: str) -> dict[str, object]:
    return require_mapping(json.loads(json.dumps(value)), label)


def require_mapping(value: object, label: str) -> dict[str, object]:
    if not isinstance(value, dict):
        raise DriftCheckError("FAIL", f"{label} is not a JSON object")
    return value


def require_exact_keys(value: dict[str, object], expected: frozenset[str], label: str) -> None:
    actual = set(value)
    if actual != expected:
        raise DriftCheckError(
            "FAIL",
            f"{label} has unexpected keys: expected {sorted(expected)}, found {sorted(actual)}",
        )


def require_bool(value: object, label: str) -> bool:
    if not isinstance(value, bool):
        raise DriftCheckError("FAIL", f"{label} is not a JSON boolean: {value!r}")
    return value


def require_string(value: object, label: str) -> str:
    if not isinstance(value, str):
        raise DriftCheckError("FAIL", f"{label} is not a JSON string: {value!r}")
    return value


def require_string_list(value: object, label: str) -> list[str]:
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        raise DriftCheckError("FAIL", f"{label} is not a string array: {value!r}")
    return list(value)


def require_git_sha(value: object, label: str) -> str:
    if not isinstance(value, str) or not GIT_SHA_RE.fullmatch(value):
        raise DriftCheckError("FAIL", f"{label} is not a valid git object id: {value!r}")
    return value


def require_file_sha256(value: object, label: str) -> str:
    if not isinstance(value, str) or not SHA256_RE.fullmatch(value):
        raise DriftCheckError("FAIL", f"{label} is not a valid SHA-256 digest: {value!r}")
    return value


def git(args: list[str], *, failure_status: str = "BLOCKED") -> str:
    try:
        result = subprocess.run(
            ["git", "-C", str(ROOT), *args],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        raise DriftCheckError(failure_status, f"git {' '.join(args[:2])} failed") from exc
    return result.stdout.strip()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def evaluate_closure(
    closure: dict[str, object],
    provenance: dict[str, object],
    head_sha: str,
    changed_paths: list[str] | None,
    provenance_dir: Path,
) -> None:
    """Raise DriftCheckError unless the closure holds for this repository state."""
    if closure.get("schema") != EXPECTED_CLOSURE_SCHEMA:
        raise DriftCheckError(
            "FAIL",
            f"unexpected closure schema: {closure.get('schema')!r} != {EXPECTED_CLOSURE_SCHEMA!r}",
        )
    if provenance.get("schema") != EXPECTED_PROVENANCE_SCHEMA:
        raise DriftCheckError(
            "FAIL",
            "unexpected import provenance schema: "
            f"{provenance.get('schema')!r} != {EXPECTED_PROVENANCE_SCHEMA!r}",
        )

    # The grant itself must agree on both records before anything else counts.
    if closure.get("MASTER_DRIFT") != "PASS":
        raise DriftCheckError(
            "FAIL",
            f"closure does not grant MASTER_DRIFT = PASS (found {closure.get('MASTER_DRIFT')!r})",
        )
    baseline_identity = require_mapping(
        provenance.get("baseline_identity"), "provenance.baseline_identity"
    )
    if baseline_identity.get("master_drift_gate") != "PASS":
        raise DriftCheckError(
            "FAIL",
            "import provenance does not record MASTER_DRIFT = PASS "
            f"(found {baseline_identity.get('master_drift_gate')!r})",
        )
    if closure.get("MASTER_DRIFT") != baseline_identity.get("master_drift_gate"):
        raise DriftCheckError("FAIL", "closure and provenance disagree about the gate grant")

    verified_map = require_mapping(closure.get("verified_master"), "closure.verified_master")
    verified_sha = require_git_sha(verified_map.get("sha256"), "closure verified SHA")
    provenance_verified = require_git_sha(
        baseline_identity.get("verified_current_master_sha_at_import"),
        "provenance verified current master SHA",
    )
    if verified_sha != provenance_verified:
        raise DriftCheckError(
            "FAIL",
            "closure verified master "
            f"{verified_sha} contradicts provenance verified master {provenance_verified}; "
            "the closure identity has been substituted",
        )

    closure_baseline_map = require_mapping(closure.get("rev3_baseline"), "closure.rev3_baseline")
    baseline_sha = require_git_sha(
        closure_baseline_map.get("recorded_repository_sha"), "closure baseline SHA"
    )
    provenance_baseline = require_git_sha(
        baseline_identity.get("rev3_recorded_repository_sha"),
        "provenance REV3 baseline SHA",
    )
    if baseline_sha != provenance_baseline:
        raise DriftCheckError(
            "FAIL",
            "closure baseline "
            f"{baseline_sha} contradicts provenance baseline {provenance_baseline}",
        )

    package_zip_map = require_mapping(provenance.get("source_package"), "provenance.source_package")
    provenance_zip = package_zip_map.get("sha256")
    closure_zip = closure_baseline_map.get("package_zip_sha256")
    if not isinstance(provenance_zip, str) or provenance_zip != closure_zip:
        raise DriftCheckError(
            "FAIL",
            "closure and import provenance disagree about the source package digest",
        )
    require_file_sha256(provenance_zip, "source package digest")

    # Identity ancestry is a property of the two pinned commits themselves and
    # is therefore checked on every evaluation, independent of HEAD.
    git(["merge-base", "--is-ancestor", baseline_sha, verified_sha], failure_status="FAIL")

    if changed_paths is None:
        if head_sha != verified_sha:
            raise DriftCheckError(
                "FAIL",
                f"repository HEAD {head_sha} does not match verified master {verified_sha}",
            )
    else:
        outside = sorted(path for path in changed_paths if not path_is_allowed(path))
        if outside:
            raise DriftCheckError(
                "FAIL",
                "normative-surface drift since the verified master; MASTER_DRIFT must be "
                f"re-evaluated for: {outside[:10]}",
            )
        if head_sha != verified_sha:
            git(
                ["merge-base", "--is-ancestor", verified_sha, head_sha],
                failure_status="FAIL",
            )

    imported_map = require_mapping(provenance.get("import_boundary"), "provenance.import_boundary")
    imported_files_map = require_mapping(
        imported_map.get("imported_files"),
        "provenance.import_boundary.imported_files",
    )
    covered: set[str] = set()
    for relative, expected_digest in sorted(imported_files_map.items()):
        require_file_sha256(expected_digest, f"recorded digest for {relative!r}")
        candidate = provenance_dir / str(relative)
        if not candidate.is_file():
            raise DriftCheckError("FAIL", f"promoted evidence file missing: {candidate}")
        actual = sha256_file(candidate)
        if actual != expected_digest:
            raise DriftCheckError(
                "FAIL",
                "promoted evidence mutated since import: "
                f"{relative} ({actual} != {expected_digest})",
            )
        covered.add(str(relative))

    # The closure record binds its sibling top-level records by digest so the
    # claim they are unmodified is enforced, not just asserted.
    bound_records = require_mapping(closure.get("bound_records"), "closure.bound_records")
    for relative in (PROVENANCE_FILENAME, REPORT_FILENAME):
        expected = require_file_sha256(
            bound_records.get(relative), f"closure bound record {relative}"
        )
        candidate = provenance_dir / relative
        if not candidate.is_file():
            raise DriftCheckError("FAIL", f"bound record missing: {candidate}")
        actual = sha256_file(candidate)
        if actual != expected:
            raise DriftCheckError(
                "FAIL",
                f"bound record mutated since closure: {relative} ({actual} != {expected})",
            )
        covered.add(relative)

    for path in sorted(provenance_dir.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(provenance_dir).as_posix()
        if (
            relative not in covered
            and relative != CLOSURE_FILENAME
            and relative not in ADDITIVE_REVALIDATION_FILES
        ):
            raise DriftCheckError(
                "FAIL",
                f"evidence file present but not digest-recorded: {relative}",
            )


def validate_historical_records(
    closure: dict[str, object], provenance: dict[str, object], provenance_dir: Path
) -> str:
    """Validate the original closure without treating it as a current-master pointer."""
    verified_map = require_mapping(closure.get("verified_master"), "closure.verified_master")
    verified_sha = require_git_sha(verified_map.get("sha256"), "verified SHA")
    evaluate_closure(closure, provenance, verified_sha, None, provenance_dir)
    return verified_sha


def require_bound_file(
    binding: object,
    label: str,
    expected_path: str,
    expected_file: Path,
) -> str:
    record = require_mapping(binding, label)
    require_exact_keys(record, frozenset({"path", "raw_sha256"}), label)
    path = require_string(record.get("path"), f"{label}.path")
    if path != expected_path:
        raise DriftCheckError("FAIL", f"{label}.path is {path!r}, expected {expected_path!r}")
    expected = require_file_sha256(record.get("raw_sha256"), f"{label}.raw_sha256")
    if not expected_file.is_file():
        raise DriftCheckError("FAIL", f"bound evidence file missing: {expected_file}")
    actual = sha256_file(expected_file)
    if actual != expected:
        raise DriftCheckError("FAIL", f"bound evidence mutated: {path} ({actual} != {expected})")
    return expected


def validate_revalidation(
    revalidation: dict[str, object],
    closure: dict[str, object],
    provenance: dict[str, object],
    provenance_dir: Path,
) -> str:
    """Validate the additive post-#83 revalidation authority."""
    require_exact_keys(revalidation, REVALIDATION_TOP_LEVEL_KEYS, REVALIDATION_FILENAME)
    if revalidation.get("schema") != EXPECTED_REVALIDATION_SCHEMA:
        raise DriftCheckError(
            "FAIL",
            f"unexpected revalidation schema: {revalidation.get('schema')!r} != "
            f"{EXPECTED_REVALIDATION_SCHEMA!r}",
        )
    if revalidation.get("revalidation_id") != "m2.5.a.master-drift-revalidation.1a1e504":
        raise DriftCheckError("FAIL", "unexpected revalidation identity")

    historical = require_mapping(
        revalidation.get("historical_identity"), "revalidation.historical_identity"
    )
    require_exact_keys(historical, HISTORICAL_IDENTITY_KEYS, "revalidation.historical_identity")
    closure_baseline = require_mapping(closure.get("rev3_baseline"), "closure.rev3_baseline")
    closure_verified = require_mapping(closure.get("verified_master"), "closure.verified_master")
    provenance_baseline = require_mapping(
        require_mapping(provenance.get("baseline_identity"), "provenance.baseline_identity"),
        "provenance.baseline_identity",
    )
    original_recorded = require_git_sha(
        historical.get("original_rev3_recorded_repository_sha"),
        "revalidation original recorded repository SHA",
    )
    original_import_verified = require_git_sha(
        historical.get("original_import_verified_master_sha"),
        "revalidation original import verified SHA",
    )
    if original_recorded != require_git_sha(
        closure_baseline.get("recorded_repository_sha"), "closure baseline SHA"
    ) or original_recorded != require_git_sha(
        provenance_baseline.get("rev3_recorded_repository_sha"),
        "provenance baseline SHA",
    ):
        raise DriftCheckError(
            "FAIL", "revalidation does not preserve the historical REV3 baseline SHA"
        )
    if original_import_verified != require_git_sha(
        closure_verified.get("sha256"), "closure verified SHA"
    ) or original_import_verified != require_git_sha(
        provenance_baseline.get("verified_current_master_sha_at_import"),
        "provenance verified current master SHA",
    ):
        raise DriftCheckError(
            "FAIL", "revalidation does not preserve the historical import-time verified SHA"
        )

    require_bound_file(
        historical.get("original_closure"),
        "revalidation.historical_identity.original_closure",
        f"sources/m2_5/pre_research/REV3/{CLOSURE_FILENAME}",
        provenance_dir / CLOSURE_FILENAME,
    )
    require_bound_file(
        historical.get("original_import_provenance"),
        "revalidation.historical_identity.original_import_provenance",
        f"sources/m2_5/pre_research/REV3/{PROVENANCE_FILENAME}",
        provenance_dir / PROVENANCE_FILENAME,
    )
    require_bound_file(
        historical.get("original_report"),
        "revalidation.historical_identity.original_report",
        f"sources/m2_5/pre_research/REV3/{REPORT_FILENAME}",
        provenance_dir / REPORT_FILENAME,
    )
    package = require_mapping(provenance.get("source_package"), "provenance.source_package")
    package_binding = require_mapping(
        historical.get("rev3_package"), "revalidation.historical_identity.rev3_package"
    )
    require_exact_keys(
        package_binding,
        frozenset({"logical_locator", "raw_sha256"}),
        "revalidation.historical_identity.rev3_package",
    )
    if package_binding.get("logical_locator") != package.get("logical_locator"):
        raise DriftCheckError("FAIL", "revalidation package locator differs from import provenance")
    package_sha = require_file_sha256(package.get("sha256"), "provenance source package digest")
    if (
        require_file_sha256(package_binding.get("raw_sha256"), "revalidation package digest")
        != package_sha
    ):
        raise DriftCheckError("FAIL", "revalidation package digest differs from import provenance")

    reviewed = require_mapping(revalidation.get("reviewed_range"), "revalidation.reviewed_range")
    require_exact_keys(reviewed, REVIEWED_RANGE_KEYS, "revalidation.reviewed_range")
    previous = require_git_sha(
        reviewed.get("previous_effective_passing_head"),
        "revalidation previous effective passing head",
    )
    new_master = require_git_sha(
        reviewed.get("new_revalidated_master"), "revalidation new revalidated master"
    )
    merge_commit = require_git_sha(reviewed.get("merge_commit"), "revalidation merge commit")
    merge_parents = require_string_list(reviewed.get("merge_parents"), "revalidation merge parents")
    repair_head = require_git_sha(reviewed.get("repair_head"), "revalidation repair head")
    repair_parent = require_git_sha(reviewed.get("repair_parent"), "revalidation repair parent")
    if (previous, new_master, merge_commit, repair_head, repair_parent) != (
        EXPECTED_PREVIOUS_EFFECTIVE_HEAD,
        EXPECTED_NEW_REVALIDATED_MASTER,
        EXPECTED_NEW_REVALIDATED_MASTER,
        EXPECTED_REPAIR_HEAD,
        EXPECTED_REPAIR_PARENT,
    ):
        raise DriftCheckError(
            "FAIL", "revalidation reviewed heads do not match the approved #83 range"
        )
    if merge_parents != [EXPECTED_PREVIOUS_EFFECTIVE_HEAD, EXPECTED_REPAIR_HEAD]:
        raise DriftCheckError(
            "FAIL", "revalidation merge parents do not match the approved #83 merge"
        )
    if require_string(reviewed.get("reviewed_commit_range"), "reviewed commit range") != (
        f"{EXPECTED_PREVIOUS_EFFECTIVE_HEAD}..{EXPECTED_NEW_REVALIDATED_MASTER}"
    ):
        raise DriftCheckError("FAIL", "revalidation commit range is not the exact post-#83 range")
    if (
        require_string_list(reviewed.get("reviewed_commit_shas"), "reviewed commit SHAs")
        != EXPECTED_REVIEWED_COMMITS
    ):
        raise DriftCheckError(
            "FAIL", "revalidation commit list differs from the exact post-#83 range"
        )
    changed_paths = require_string_list(reviewed.get("changed_paths"), "revalidation changed paths")
    if changed_paths != EXPECTED_REVIEWED_PATHS:
        raise DriftCheckError(
            "FAIL", f"revalidation changed paths differ from the reviewed set: {changed_paths!r}"
        )
    actual_merge = git(["rev-list", "--parents", "-n", "1", new_master]).split()
    if actual_merge != [new_master, previous, repair_head]:
        expected_merge = [new_master, previous, repair_head]
        raise DriftCheckError(
            "FAIL",
            f"new master merge ancestry is {actual_merge!r}, expected {expected_merge!r}",
        )
    actual_repair = git(["rev-list", "--parents", "-n", "1", repair_head]).split()
    if actual_repair != [repair_head, repair_parent]:
        raise DriftCheckError(
            "FAIL",
            f"repair ancestry is {actual_repair!r}, expected {[repair_head, repair_parent]!r}",
        )
    actual_commits = git(["rev-list", "--reverse", f"{previous}..{new_master}"]).splitlines()
    if actual_commits != EXPECTED_REVIEWED_COMMITS:
        raise DriftCheckError(
            "FAIL",
            f"actual reviewed commits are {actual_commits!r}, "
            f"expected {EXPECTED_REVIEWED_COMMITS!r}",
        )
    actual_paths = git(["diff", "--name-only", f"{previous}..{new_master}"]).splitlines()
    if actual_paths != EXPECTED_REVIEWED_PATHS:
        raise DriftCheckError(
            "FAIL",
            f"actual reviewed paths are {actual_paths!r}, expected {EXPECTED_REVIEWED_PATHS!r}",
        )
    git(["merge-base", "--is-ancestor", previous, new_master], failure_status="FAIL")
    git(["merge-base", "--is-ancestor", repair_head, new_master], failure_status="FAIL")

    normative = require_mapping(
        revalidation.get("normative_drift_review"), "revalidation.normative_drift_review"
    )
    require_exact_keys(normative, NORMATIVE_REVIEW_KEYS, "revalidation.normative_drift_review")
    control_paths = require_string_list(normative.get("control_paths"), "normative control paths")
    unchanged_paths = require_string_list(
        normative.get("unchanged_paths"), "unchanged normative paths"
    )
    expected_controls = list(NORMATIVE_DRIFT_CONTROL_PATHS)
    if control_paths != expected_controls or unchanged_paths != expected_controls:
        raise DriftCheckError(
            "FAIL", "revalidation normative control inventory is incomplete or reordered"
        )
    for path in expected_controls:
        if git(["diff", "--name-only", f"{previous}..{new_master}", "--", path]):
            raise DriftCheckError(
                "FAIL", f"normative control path changed in reviewed range: {path}"
            )

    semantic = require_mapping(
        revalidation.get("semantic_impact_review"), "revalidation.semantic_impact_review"
    )
    require_exact_keys(semantic, SEMANTIC_IMPACT_KEYS, "revalidation.semantic_impact_review")
    if require_bool(semantic.get("material_semantic_drift"), "material semantic drift"):
        raise DriftCheckError("FAIL", "revalidation records material semantic drift")
    if semantic.get("impact_classification") != "TEST_INFRASTRUCTURE_ONLY":
        raise DriftCheckError(
            "FAIL", "revalidation impact classification is not TEST_INFRASTRUCTURE_ONLY"
        )
    surfaces = require_mapping(semantic.get("surface_results"), "semantic impact surface results")
    require_exact_keys(surfaces, SEMANTIC_SURFACE_KEYS, "semantic impact surface results")
    for key, result in surfaces.items():
        expected_result = (
            "AFFECTED_REVIEWED" if key == "python_test_infrastructure" else "UNAFFECTED"
        )
        if result != expected_result:
            raise DriftCheckError("FAIL", f"semantic impact result for {key} is {result!r}")

    outcome = require_mapping(revalidation.get("outcome"), "revalidation.outcome")
    require_exact_keys(outcome, OUTCOME_KEYS, "revalidation.outcome")
    if outcome.get("MASTER_DRIFT") != "PASS":
        raise DriftCheckError("FAIL", "revalidation does not grant MASTER_DRIFT = PASS")
    if not require_bool(outcome.get("rev3_evidence_reusable"), "REV3 evidence reusable"):
        raise DriftCheckError("FAIL", "revalidation does not preserve REV3 evidence reuse")
    if require_bool(
        outcome.get("research_data_revalidation_required"),
        "research-data revalidation required",
    ):
        raise DriftCheckError("FAIL", "revalidation requires research-data regeneration")
    if outcome.get("effective_descendant_anchor") != EXPECTED_NEW_REVALIDATED_MASTER:
        raise DriftCheckError("FAIL", "revalidation effective descendant anchor is incorrect")

    bound = require_mapping(revalidation.get("bound_evidence"), "revalidation.bound_evidence")
    require_exact_keys(bound, BOUND_EVIDENCE_KEYS, "revalidation.bound_evidence")
    require_bound_file(
        bound.get("revalidation_report"),
        "revalidation.bound_evidence.revalidation_report",
        f"sources/m2_5/pre_research/REV3/{REVALIDATION_REPORT_FILENAME}",
        provenance_dir / REVALIDATION_REPORT_FILENAME,
    )
    return new_master


def evaluate_descendant_drift(anchor_sha: str, head_sha: str, changed_paths: list[str]) -> None:
    outside = sorted(path for path in changed_paths if not path_is_allowed(path))
    if outside:
        raise DriftCheckError(
            "FAIL",
            "normative-surface drift since the effective revalidation anchor; "
            f"MASTER_DRIFT must be re-evaluated for: {outside[:10]}",
        )
    if head_sha != anchor_sha:
        git(["merge-base", "--is-ancestor", anchor_sha, head_sha], failure_status="FAIL")


def collect_changed_paths(verified_sha: str, head_sha: str = "HEAD") -> list[str]:
    output = git(["diff", "--name-only", f"{verified_sha}..{head_sha}"])
    return [line for line in output.splitlines() if line.strip()]


def validate_historical_chain(
    closure: dict[str, object], provenance: dict[str, object], provenance_dir: Path
) -> str:
    """Validate the original closure through the previous effective head."""
    historical_anchor = validate_historical_records(closure, provenance, provenance_dir)
    previous_changed_paths = collect_changed_paths(
        historical_anchor, EXPECTED_PREVIOUS_EFFECTIVE_HEAD
    )
    evaluate_closure(
        closure,
        provenance,
        EXPECTED_PREVIOUS_EFFECTIVE_HEAD,
        previous_changed_paths,
        provenance_dir,
    )
    return historical_anchor


def run_check(provenance_dir: Path, expect_head: str | None) -> int:
    closure = require_mapping(read_json(provenance_dir / CLOSURE_FILENAME), CLOSURE_FILENAME)
    provenance = require_mapping(
        read_json(provenance_dir / PROVENANCE_FILENAME), PROVENANCE_FILENAME
    )
    validate_historical_chain(closure, provenance, provenance_dir)
    revalidation = load_revalidation(provenance_dir)
    effective_anchor = validate_revalidation(revalidation, closure, provenance, provenance_dir)
    head_sha = require_git_sha(
        expect_head if expect_head is not None else git(["rev-parse", "HEAD"]),
        "repository HEAD",
    )
    changed_paths = collect_changed_paths(effective_anchor, head_sha)
    evaluate_descendant_drift(effective_anchor, head_sha, changed_paths)
    print(
        "MASTER_DRIFT_REVALIDATION_CHECK = PASS "
        f"(effective anchor {effective_anchor}; head {head_sha})"
    )
    return EXIT_PASS


def verify_archive(provenance_dir: Path) -> int:
    """Preflight the private archive contract: exists AND exact SHA, else fail."""
    provenance = require_mapping(
        read_json(provenance_dir / PROVENANCE_FILENAME), PROVENANCE_FILENAME
    )
    closure = require_mapping(read_json(provenance_dir / CLOSURE_FILENAME), CLOSURE_FILENAME)
    validate_historical_chain(closure, provenance, provenance_dir)
    validate_revalidation(load_revalidation(provenance_dir), closure, provenance, provenance_dir)
    package = require_mapping(provenance.get("source_package"), "provenance.source_package")
    storage_class = package.get("storage_class")
    if storage_class != "MAINTAINER_PRIVATE_ARCHIVE":
        raise DriftCheckError("FAIL", f"unexpected archive storage class: {storage_class!r}")
    locator_template = package.get("logical_locator")
    if not isinstance(locator_template, str) or ARCHIVE_ENV_VAR not in locator_template:
        raise DriftCheckError("FAIL", f"malformed logical locator: {locator_template!r}")
    base = os.environ.get(ARCHIVE_ENV_VAR)
    if not base:
        raise DriftCheckError(
            "BLOCKED",
            f"environment variable {ARCHIVE_ENV_VAR} is unset; the maintainer-private "
            "archive location is unknown and excluded payload cannot be located",
        )
    relative = locator_template.replace(f"${{{ARCHIVE_ENV_VAR}}}", "").replace(
        f"${ARCHIVE_ENV_VAR}", ""
    )
    archive = Path(base) / relative.lstrip("/")
    if not archive.is_file():
        raise DriftCheckError(
            "BLOCKED",
            f"archive not found at resolved locator {archive}; consuming slices are BLOCKED",
        )
    expected = require_file_sha256(package.get("sha256"), "archive sha256")
    actual = sha256_file(archive)
    if actual != expected:
        raise DriftCheckError(
            "FAIL",
            f"archive digest mismatch at {archive} ({actual} != {expected})",
        )
    print(f"ARCHIVE_PREFLIGHT = PASS ({archive})")
    return EXIT_PASS


def negative_self_test(provenance_dir: Path) -> int:
    """Prove stale/mismatched/tampered identities can never silently receive PASS."""
    closure = require_mapping(
        json.loads((provenance_dir / CLOSURE_FILENAME).read_text("utf-8")), CLOSURE_FILENAME
    )
    provenance = require_mapping(
        json.loads((provenance_dir / PROVENANCE_FILENAME).read_text("utf-8")),
        PROVENANCE_FILENAME,
    )
    revalidation = load_revalidation(provenance_dir)
    live_head = git(["rev-parse", "HEAD"])

    def tampered_closure(mutate: Callable[[dict[str, object]], None]) -> dict[str, object]:
        value = clone_mapping(closure, CLOSURE_FILENAME)
        mutate(value)
        return value

    def set_nested(mapping: dict[str, object], parent: str, key: str, value: object) -> None:
        require_mapping(mapping.get(parent), parent)[key] = value

    def substitute_valid_sha() -> None:
        # A syntactically valid 40-hex SHA that is NOT the provenance-pinned
        # verified master: the exact false-PASS shape this checker must reject.
        substituted = tampered_closure(
            lambda value: set_nested(value, "verified_master", "sha256", live_head)
        )
        evaluate_closure(substituted, provenance, live_head, [], provenance_dir)

    def stale_head() -> None:
        evaluate_closure(closure, provenance, "0" * 40, None, provenance_dir)

    def normative_drift() -> None:
        rejected_paths = []
        for controlled in NORMATIVE_DRIFT_CONTROL_PATHS:
            try:
                evaluate_closure(
                    closure,
                    provenance,
                    live_head,
                    [controlled],
                    provenance_dir,
                )
            except DriftCheckError:
                rejected_paths.append(controlled)
            else:
                raise DriftCheckError(
                    "FAIL",
                    f"normative path {controlled!r} did not invalidate the closure",
                )
        if len(rejected_paths) != len(NORMATIVE_DRIFT_CONTROL_PATHS):
            raise DriftCheckError("FAIL", "normative drift control coverage incomplete")
        # Every controlled path was individually proven to break the closure;
        # surface this to the harness as the fixture's expected rejection.
        raise DriftCheckError(
            "FAIL",
            "expected rejection: all "
            f"{len(rejected_paths)} normative control paths invalidated the closure",
        )

    def near_miss_rejected(path: str) -> None:
        if path_is_allowed(path):
            raise AssertionError(f"near-miss path was incorrectly allowed: {path}")
        raise DriftCheckError(
            "FAIL",
            f"near-miss path correctly rejected by path_is_allowed: {path}",
            code="ALLOWLIST_NEAR_MISS_PATH_REJECTED",
        )

    def non_pass_grant() -> None:
        downgraded = tampered_closure(lambda value: value.__setitem__("MASTER_DRIFT", "FAIL"))
        evaluate_closure(downgraded, provenance, live_head, [], provenance_dir)

    def tampered_verified_sha_invalid() -> None:
        malformed = tampered_closure(
            lambda value: set_nested(value, "verified_master", "sha256", "f" * 64)
        )
        evaluate_closure(malformed, provenance, "e" * 40 + "ff", None, provenance_dir)

    def tampered_provenance_verified_sha() -> None:
        flipped = clone_mapping(provenance, PROVENANCE_FILENAME)
        set_nested(flipped, "baseline_identity", "verified_current_master_sha_at_import", "a" * 40)
        evaluate_closure(closure, flipped, live_head, [], provenance_dir)

    def tampered_baseline_sha() -> None:
        rewritten_history = tampered_closure(
            lambda value: set_nested(value, "rev3_baseline", "recorded_repository_sha", "b" * 40)
        )
        evaluate_closure(rewritten_history, provenance, live_head, [], provenance_dir)

    def wrong_schema() -> None:
        old_schema = tampered_closure(
            lambda value: value.__setitem__("schema", "manafold.m2.5.a.master-drift-closure.v0")
        )
        evaluate_closure(old_schema, provenance, live_head, [], provenance_dir)

    def unbound_sibling_edit() -> None:
        edited_report = provenance_dir / REPORT_FILENAME
        original = edited_report.read_bytes()
        try:
            edited_report.write_bytes(original + b"<!-- tampered -->\n")
            evaluate_closure(closure, provenance, live_head, [], provenance_dir)
        finally:
            edited_report.write_bytes(original)

    def unrecorded_import() -> None:
        stripped = json.loads(json.dumps(provenance))
        first_imported = next(iter(stripped["import_boundary"]["imported_files"]))
        del stripped["import_boundary"]["imported_files"][first_imported]
        evaluate_closure(closure, stripped, live_head, [], provenance_dir)

    def tampered_revalidation(mutate: Callable[[dict[str, object]], None]) -> dict[str, object]:
        value = clone_mapping(revalidation, REVALIDATION_FILENAME)
        mutate(value)
        return value

    def validate_additive(candidate: dict[str, object]) -> None:
        validate_historical_chain(closure, provenance, provenance_dir)
        validate_revalidation(candidate, closure, provenance, provenance_dir)

    def previous_effective_bridge_rejects_unallowed_path() -> None:
        original_collect_changed_paths = globals().get("collect_changed_paths")
        try:
            globals()["collect_changed_paths"] = lambda _verified_sha, _head_sha: ["pytest.ini"]
            validate_historical_chain(closure, provenance, provenance_dir)
        finally:
            if original_collect_changed_paths is None:
                globals().pop("collect_changed_paths", None)
            else:
                globals()["collect_changed_paths"] = original_collect_changed_paths

    def original_import_time_sha_substitution() -> None:
        substituted = clone_mapping(provenance, PROVENANCE_FILENAME)
        set_nested(
            substituted, "baseline_identity", "verified_current_master_sha_at_import", "c" * 40
        )
        validate_historical_records(closure, substituted, provenance_dir)

    def original_closure_tamper() -> None:
        tampered = tampered_closure(
            lambda value: set_nested(value, "bound_records", REPORT_FILENAME, "d" * 64)
        )
        validate_historical_records(tampered, provenance, provenance_dir)

    def previous_effective_head_substitution() -> None:
        candidate = tampered_revalidation(
            lambda value: set_nested(
                value, "reviewed_range", "previous_effective_passing_head", "0" * 40
            )
        )
        validate_additive(candidate)

    def new_verified_master_substitution() -> None:
        candidate = tampered_revalidation(
            lambda value: set_nested(value, "reviewed_range", "new_revalidated_master", "e" * 40)
        )
        validate_additive(candidate)

    def missing_reviewed_path() -> None:
        candidate = tampered_revalidation(
            lambda value: set_nested(
                value, "reviewed_range", "changed_paths", EXPECTED_REVIEWED_PATHS[1:]
            )
        )
        validate_additive(candidate)

    def extra_unreviewed_path() -> None:
        candidate = tampered_revalidation(
            lambda value: set_nested(
                value,
                "reviewed_range",
                "changed_paths",
                [*EXPECTED_REVIEWED_PATHS, "unreviewed-fourth-path"],
            )
        )
        validate_additive(candidate)

    def stale_merge_ancestry() -> None:
        candidate = tampered_revalidation(
            lambda value: set_nested(
                value,
                "reviewed_range",
                "merge_parents",
                [EXPECTED_PREVIOUS_EFFECTIVE_HEAD, EXPECTED_REPAIR_PARENT],
            )
        )
        validate_additive(candidate)

    def tampered_revalidation_report_digest() -> None:
        candidate = tampered_revalidation(
            lambda value: set_nested(
                require_mapping(value.get("bound_evidence"), "bound_evidence"),
                "revalidation_report",
                "raw_sha256",
                "f" * 64,
            )
        )
        validate_additive(candidate)

    def future_repair_paths_rejected() -> None:
        for path in EXPECTED_REVIEWED_PATHS:
            if path_is_allowed(path):
                raise AssertionError(f"reviewed repair path was future-allowlisted: {path}")
        raise DriftCheckError(
            "FAIL",
            "reviewed repair paths correctly remain outside the future descendant allowlist",
            code="REVALIDATION_PATHS_NOT_FUTURE_ALLOWLISTED",
        )

    def additive_revalidation_is_required() -> None:
        validate_revalidation(
            tampered_revalidation(
                lambda value: set_nested(value, "outcome", "MASTER_DRIFT", "FAIL")
            ),
            closure,
            provenance,
            provenance_dir,
        )

    cases: list[tuple[str, str, Callable[[], None], str | None]] = []

    def expect_failure(
        case_id: str,
        reason: str,
        thunk: Callable[[], None],
        expected_code: str | None = None,
    ) -> None:
        cases.append((case_id, reason, thunk, expected_code))

    expect_failure(
        "ALLOWLIST_NEAR_MISS_B2_SUFFIX_REJECTED",
        "the exact B2 checker path must not receive an implicit suffix match",
        lambda: near_miss_rejected("scripts/check_m2_5_b2_classifications.py.backup"),
        "ALLOWLIST_NEAR_MISS_PATH_REJECTED",
    )
    expect_failure(
        "ALLOWLIST_NEAR_MISS_B1_SUFFIX_REJECTED",
        "the exact B1 checker path must not receive an implicit suffix match",
        lambda: near_miss_rejected("scripts/check_m2_5_b1_authority_citations.py.backup"),
        "ALLOWLIST_NEAR_MISS_PATH_REJECTED",
    )
    expect_failure(
        "ALLOWLIST_NEAR_MISS_B1_FINAL_SUFFIX_REJECTED",
        "the exact B1.Final checker path must not receive an implicit suffix match",
        lambda: near_miss_rejected("scripts/check_m2_5_b1_final_authority_citations.py.backup"),
        "ALLOWLIST_NEAR_MISS_PATH_REJECTED",
    )
    expect_failure(
        "ALLOWLIST_NEAR_MISS_REV3_PREFIX_REJECTED",
        "the REV3 directory prefix must not match a sibling directory",
        lambda: near_miss_rejected("sources/m2_5/closures/B20/foo"),
        "ALLOWLIST_NEAR_MISS_PATH_REJECTED",
    )
    expect_failure(
        "ALLOWLIST_NEAR_MISS_B1_PREFIX_REJECTED",
        "the B1 directory prefix must not match a sibling directory",
        lambda: near_miss_rejected("sources/m2_5/closures/B10/foo"),
        "ALLOWLIST_NEAR_MISS_PATH_REJECTED",
    )

    expect_failure(
        "SUBSTITUTED_VALID_SHA_REJECTED",
        "a substituted but syntactically valid verified SHA (current HEAD) must FAIL",
        substitute_valid_sha,
    )
    expect_failure(
        "STALE_HEAD_REJECTED",
        "an unrelated HEAD must never receive PASS",
        stale_head,
    )
    expect_failure(
        "NORMATIVE_DRIFT_REJECTED",
        "post-verification commits touching normative paths must never receive PASS",
        normative_drift,
    )
    expect_failure(
        "NON_PASS_GRANT_REJECTED",
        "a closure that does not grant PASS must never receive PASS",
        non_pass_grant,
    )
    expect_failure(
        "TAMPERED_VERIFIED_SHA_INVALID_REJECTED",
        "a malformed verified SHA must never receive PASS",
        tampered_verified_sha_invalid,
    )
    expect_failure(
        "TAMPERED_PROVENANCE_VERIFIED_SHA_REJECTED",
        "editing the provenance-side verified master must contradict the closure and FAIL",
        tampered_provenance_verified_sha,
    )
    expect_failure(
        "TAMPERED_BASELINE_SHA_REJECTED",
        "rewriting the REV3 baseline SHA breaks recorded ancestry and must FAIL",
        tampered_baseline_sha,
    )
    expect_failure(
        "WRONG_SCHEMA_REJECTED",
        "an unrecognized closure schema must never receive PASS",
        wrong_schema,
    )
    expect_failure(
        "UNBOUND_SIBLING_EDIT_REJECTED",
        "digest-bound siblings must be rejected on any edit without reclosure",
        unbound_sibling_edit,
    )
    expect_failure(
        "UNRECORDED_IMPORT_REJECTED",
        "every promoted evidence file must stay digest-recorded",
        unrecorded_import,
    )
    expect_failure(
        "ORIGINAL_IMPORT_TIME_SHA_SUBSTITUTION_REJECTED",
        "historical import-time master identity is immutable and cannot be substituted",
        original_import_time_sha_substitution,
    )
    expect_failure(
        "ORIGINAL_CLOSURE_TAMPER_REJECTED",
        "the original closure remains digest-bound historical evidence",
        original_closure_tamper,
    )
    expect_failure(
        "REVALIDATION_PREVIOUS_EFFECTIVE_HEAD_SUBSTITUTION_REJECTED",
        "the revalidation must name the exact previous effective passing head",
        previous_effective_head_substitution,
    )
    expect_failure(
        "HISTORICAL_TO_PREVIOUS_EFFECTIVE_BRIDGE_REJECTED",
        "the historical closure must reject an unallowed path before the additive revalidation",
        previous_effective_bridge_rejects_unallowed_path,
    )
    expect_failure(
        "REVALIDATION_NEW_MASTER_SUBSTITUTION_REJECTED",
        "the revalidation must name the exact new verified master",
        new_verified_master_substitution,
    )
    expect_failure(
        "REVALIDATION_MISSING_REVIEWED_PATH_REJECTED",
        "the exact three-path reviewed repair range must be complete",
        missing_reviewed_path,
    )
    expect_failure(
        "REVALIDATION_EXTRA_REVIEWED_PATH_REJECTED",
        "an unreviewed fourth path must invalidate the revalidation",
        extra_unreviewed_path,
    )
    expect_failure(
        "REVALIDATION_STALE_MERGE_ANCESTRY_REJECTED",
        "the evidence must retain the reviewed merge ancestry",
        stale_merge_ancestry,
    )
    expect_failure(
        "REVALIDATION_REPORT_DIGEST_TAMPER_REJECTED",
        "the additive report must remain raw-digest bound",
        tampered_revalidation_report_digest,
    )
    expect_failure(
        "REVALIDATION_PATHS_NOT_FUTURE_ALLOWLISTED",
        "the three repair paths must not become permanent descendant exemptions",
        future_repair_paths_rejected,
        "REVALIDATION_PATHS_NOT_FUTURE_ALLOWLISTED",
    )
    expect_failure(
        "REVALIDATION_NEAR_MISS_PATH_REJECTED",
        "a near-miss repair path must remain outside the exact allowlist",
        lambda: near_miss_rejected("scripts/run_python_tests.py.backup"),
        "ALLOWLIST_NEAR_MISS_PATH_REJECTED",
    )
    expect_failure(
        "ADDITIVE_REVALIDATION_REQUIRED",
        "a non-PASS additive revalidation must never advance the effective anchor",
        additive_revalidation_is_required,
    )
    with tempfile.TemporaryDirectory() as tmp:

        def missing_evidence() -> None:
            evaluate_closure(closure, provenance, live_head, [], Path(tmp))

        expect_failure(
            "MISSING_EVIDENCE_REJECTED",
            "absent evidence must be rejected, never PASS",
            missing_evidence,
        )

    failures: list[str] = []
    for case_id, reason, thunk, expected_code in cases:
        try:
            thunk()
        except DriftCheckError as exc:
            if expected_code is not None and exc.code != expected_code:
                failures.append(f"{case_id}: expected code {expected_code}, found {exc.code}")
            else:
                code_suffix = f" [{exc.code}]" if exc.code is not None else ""
                print(f"NEGATIVE {case_id}: rejected ({exc.status}){code_suffix} - {reason}")
        else:
            failures.append(f"{case_id}: check unexpectedly PASSED")
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return EXIT_FAIL
    print(
        "NEGATIVE_SELF_TEST = PASS "
        f"({len(cases)} rejection cases; no stale or mismatched identity can receive PASS)"
    )
    return EXIT_PASS


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--provenance-dir",
        type=Path,
        default=DEFAULT_PROVENANCE_DIR,
        help="directory holding the promoted REV3 provenance records",
    )
    parser.add_argument("--expect-head", help="verify against this SHA instead of live HEAD")
    parser.add_argument(
        "--verify-archive",
        action="store_true",
        help=(
            "preflight the maintainer-private archive contract: resolve "
            f"{ARCHIVE_ENV_VAR}, require the ZIP to exist and match its pinned "
            "SHA-256 exactly; BLOCKED when the variable is unset"
        ),
    )
    parser.add_argument(
        "--negative-self-test",
        action="store_true",
        help="execute adversarial negative fixtures against the real checker logic",
    )
    args = parser.parse_args()
    try:
        resolved = args.provenance_dir.resolve()
        if args.negative_self_test:
            return negative_self_test(resolved)
        if args.verify_archive:
            return verify_archive(resolved)
        return run_check(resolved, args.expect_head)
    except DriftCheckError as exc:
        print(f"{exc.status}: {exc.message}")
        return EXIT_FAIL if exc.status == "FAIL" else EXIT_BLOCKED


if __name__ == "__main__":
    raise SystemExit(main())
