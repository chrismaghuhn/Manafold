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
  - HEAD equals the verified SHA, or the verified SHA is an ancestor of HEAD
    and every commit since then touches only files inside the promoted
    provenance boundary (sources/m2_5/pre_research/REV3/) or this checker;
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
        if relative not in covered and relative != CLOSURE_FILENAME:
            raise DriftCheckError(
                "FAIL",
                f"evidence file present but not digest-recorded: {relative}",
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


def verify_archive(provenance_dir: Path) -> int:
    """Preflight the private archive contract: exists AND exact SHA, else fail."""
    provenance = require_mapping(
        read_json(provenance_dir / PROVENANCE_FILENAME), PROVENANCE_FILENAME
    )
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
    live_head = git(["rev-parse", "HEAD"])

    def tampered_closure(mutate) -> dict[str, object]:
        value = json.loads(json.dumps(closure))
        mutate(value)
        return value

    def substitute_valid_sha() -> None:
        # A syntactically valid 40-hex SHA that is NOT the provenance-pinned
        # verified master: the exact false-PASS shape this checker must reject.
        substituted = tampered_closure(
            lambda value: value["verified_master"].__setitem__("sha256", live_head)
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
            lambda value: value["verified_master"].__setitem__("sha256", "f" * 64)
        )
        evaluate_closure(malformed, provenance, "e" * 40 + "ff", None, provenance_dir)

    def tampered_provenance_verified_sha() -> None:
        flipped = json.loads(json.dumps(provenance))
        flipped["baseline_identity"]["verified_current_master_sha_at_import"] = "a" * 40
        evaluate_closure(closure, flipped, live_head, [], provenance_dir)

    def tampered_baseline_sha() -> None:
        rewritten_history = tampered_closure(
            lambda value: value["rev3_baseline"].__setitem__("recorded_repository_sha", "b" * 40)
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

    cases: list[tuple[str, str, object, str | None]] = []

    def expect_failure(
        case_id: str,
        reason: str,
        thunk: object,
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
            thunk()  # type: ignore[operator]
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
