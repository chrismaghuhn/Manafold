#!/usr/bin/env python3
"""Fail-closed MASTER_DRIFT closure checker for the imported M2.5 REV3 baseline.

The closure record sources/m2_5/pre_research/REV3/master_drift_closure_REV3.json
grants MASTER_DRIFT = PASS for one verified master SHA. This script proves that
grant still holds for the repository state under test and refuses silently
passing stale, mismatched, or tampered identities.

PASS requires all of:
  - the closure record exists, parses, and carries the expected schema;
  - the closure grants MASTER_DRIFT = PASS;
  - the verified SHA is syntactically valid and agrees with the import
    provenance about the source package digest;
  - HEAD equals the verified SHA, or the verified SHA is an ancestor of HEAD
    and every commit since then touches only files inside the promoted
    provenance boundary (sources/m2_5/pre_research/REV3/) or this checker;
  - every promoted evidence file is digest-covered by IMPORT_PROVENANCE.json
    and still matches its recorded SHA-256.

Anything else exits non-zero with a precise diagnostic (FAIL) or, when evidence
cannot be evaluated at all, exits BLOCKED.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_PROVENANCE_DIR = ROOT / "sources" / "m2_5" / "pre_research" / "REV3"
CLOSURE_FILENAME = "master_drift_closure_REV3.json"
PROVENANCE_FILENAME = "IMPORT_PROVENANCE.json"
REPORT_FILENAME = "MASTER_DRIFT_REPORT.md"
EXPECTED_CLOSURE_SCHEMA = "manafold.m2.5.a.master-drift-closure.v1"
EXPECTED_PROVENANCE_SCHEMA = "manafold.m2.5.a.import-provenance.v1"
GIT_SHA_RE = re.compile(r"^[0-9a-f]{40}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
ALLOWED_PATH_PREFIXES = (
    "sources/m2_5/pre_research/REV3/",
    "scripts/check_m2_5_master_drift.py",
)

EXIT_PASS = 0
EXIT_FAIL = 1
EXIT_BLOCKED = 2


class DriftCheckError(Exception):
    def __init__(self, status: str, message: str) -> None:
        super().__init__(message)
        self.status = status
        self.message = message


def read_json(path: Path) -> object:
    if not path.is_file():
        raise DriftCheckError("BLOCKED", f"missing required evidence file: {path}")
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise DriftCheckError("BLOCKED", f"unreadable JSON evidence {path}: {exc}") from exc


def require_mapping(value: object, label: str) -> dict[str, object]:
    if not isinstance(value, dict):
        raise DriftCheckError("FAIL", f"{label} is not a JSON object")
    return value


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
    if closure.get("MASTER_DRIFT") != "PASS":
        raise DriftCheckError(
            "FAIL",
            f"closure does not grant MASTER_DRIFT = PASS (found {closure.get('MASTER_DRIFT')!r})",
        )

    verified_map = require_mapping(closure.get("verified_master"), "closure.verified_master")
    verified_sha = require_git_sha(verified_map.get("sha256"), "closure verified SHA")

    package_zip_map = require_mapping(provenance.get("source_package"), "provenance.source_package")
    closure_baseline_map = require_mapping(closure.get("rev3_baseline"), "closure.rev3_baseline")
    provenance_zip = package_zip_map.get("zip_sha256")
    closure_zip = closure_baseline_map.get("package_zip_sha256")
    if not isinstance(provenance_zip, str) or provenance_zip != closure_zip:
        raise DriftCheckError(
            "FAIL",
            "closure and import provenance disagree about the source package digest",
        )
    require_git_sha(closure_baseline_map.get("recorded_repository_sha"), "closure baseline SHA")

    if changed_paths is None:
        if head_sha != verified_sha:
            raise DriftCheckError(
                "FAIL",
                f"repository HEAD {head_sha} does not match verified master {verified_sha}",
            )
    else:
        outside = sorted(
            path for path in changed_paths if not path.startswith(ALLOWED_PATH_PREFIXES)
        )
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
    self_recorded = {CLOSURE_FILENAME, PROVENANCE_FILENAME, REPORT_FILENAME}
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
    for path in sorted(provenance_dir.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(provenance_dir).as_posix()
        if relative not in covered and relative not in self_recorded:
            raise DriftCheckError(
                "FAIL",
                f"evidence file present but not digest-recorded in import provenance: {relative}",
            )


def collect_changed_paths(verified_sha: str) -> list[str]:
    output = git(["diff", "--name-only", f"{verified_sha}..HEAD"])
    return [line for line in output.splitlines() if line.strip()]


def run_check(provenance_dir: Path, expect_head: str | None) -> int:
    closure = require_mapping(read_json(provenance_dir / CLOSURE_FILENAME), CLOSURE_FILENAME)
    provenance = require_mapping(
        read_json(provenance_dir / PROVENANCE_FILENAME), PROVENANCE_FILENAME
    )
    if expect_head is None:
        head_sha = git(["rev-parse", "HEAD"])
        verified_map = require_mapping(closure["verified_master"], "closure.verified_master")
        changed_paths = collect_changed_paths(
            require_git_sha(verified_map["sha256"], "verified SHA")
        )
    else:
        head_sha = expect_head
        changed_paths = None
    evaluate_closure(closure, provenance, head_sha, changed_paths, provenance_dir)
    print(f"MASTER_DRIFT_CLOSURE_CHECK = PASS (head {head_sha})")
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
    verified_sha = require_git_sha(closure["verified_master"]["sha256"], "verified SHA")
    live_head = git(["rev-parse", "HEAD"])
    cases: list[tuple[str, str, object]] = []

    def expect_failure(case_id: str, reason: str, thunk: object) -> None:
        cases.append((case_id, reason, thunk))

    def stale_head() -> None:
        evaluate_closure(closure, provenance, "0" * 40, None, provenance_dir)

    def normative_drift() -> None:
        evaluate_closure(
            closure,
            provenance,
            live_head,
            ["crates/mtgml-rules/src/lib.rs"],
            provenance_dir,
        )

    def non_pass_grant() -> None:
        tampered_pass = json.loads(json.dumps(closure))
        tampered_pass["MASTER_DRIFT"] = "FAIL"
        evaluate_closure(tampered_pass, provenance, verified_sha, [], provenance_dir)

    def tampered_verified_sha() -> None:
        tampered = json.loads(json.dumps(closure))
        tampered["verified_master"]["sha256"] = "f" * 64
        evaluate_closure(tampered, provenance, "e" * 40 + "ff", None, provenance_dir)

    def wrong_schema() -> None:
        tampered_schema = json.loads(json.dumps(closure))
        tampered_schema["schema"] = "manafold.m2.5.a.master-drift-closure.v0"
        evaluate_closure(tampered_schema, provenance, verified_sha, [], provenance_dir)

    def unrecorded_import() -> None:
        stripped = json.loads(json.dumps(provenance))
        first_imported = next(iter(stripped["import_boundary"]["imported_files"]))
        del stripped["import_boundary"]["imported_files"][first_imported]
        evaluate_closure(closure, stripped, verified_sha, [], provenance_dir)

    expect_failure("STALE_HEAD_REJECTED", "an unrelated HEAD must never receive PASS", stale_head)
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
        "TAMPERED_VERIFIED_SHA_REJECTED",
        "an edited verified SHA must never receive PASS",
        tampered_verified_sha,
    )
    expect_failure(
        "WRONG_SCHEMA_REJECTED",
        "an unrecognized closure schema must never receive PASS",
        wrong_schema,
    )
    expect_failure(
        "UNRECORDED_IMPORT_REJECTED",
        "every promoted evidence file must stay digest-recorded",
        unrecorded_import,
    )
    with tempfile.TemporaryDirectory() as tmp:
        empty_dir = Path(tmp)

        def missing_evidence() -> None:
            evaluate_closure(closure, provenance, verified_sha, [], empty_dir)

        expect_failure(
            "MISSING_EVIDENCE_REJECTED",
            "absent evidence must be rejected, never PASS",
            missing_evidence,
        )

    failures: list[str] = []
    for case_id, reason, thunk in cases:
        try:
            thunk()  # type: ignore[operator]
        except DriftCheckError as exc:
            print(f"NEGATIVE {case_id}: rejected ({exc.status}) - {reason}")
        else:
            failures.append(f"{case_id}: check unexpectedly PASSED")
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return EXIT_FAIL
    print("NEGATIVE_SELF_TEST = PASS (no stale or mismatched identity can receive PASS)")
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
        "--negative-self-test",
        action="store_true",
        help="execute adversarial negative fixtures against the real checker logic",
    )
    args = parser.parse_args()
    try:
        resolved = args.provenance_dir.resolve()
        if args.negative_self_test:
            return negative_self_test(resolved)
        return run_check(resolved, args.expect_head)
    except DriftCheckError as exc:
        print(f"{exc.status}: {exc.message}")
        return EXIT_FAIL if exc.status == "FAIL" else EXIT_BLOCKED


if __name__ == "__main__":
    raise SystemExit(main())
