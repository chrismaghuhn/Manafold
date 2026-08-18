#!/usr/bin/env python3
"""Run freeze gates without mutating the reproducible source tree."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tomllib
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

sys.dont_write_bytecode = True

from build_source_archive import source_files

ROOT = Path(__file__).resolve().parents[1]
ENV = dict(os.environ)
ENV["PYTHONDONTWRITEBYTECODE"] = "1"
ENV["PYTHONPATH"] = str(ROOT / "python" / "src")
FOUNDATION = "V0.2.2 Executable Freeze & Maintainer Ergonomics"
STATUS_FILENAME = "v0.2.2-status.json"
OUTPUT_MARKER = ".mtgml-verification-output"


def source_tree_fingerprint() -> str:
    hasher = hashlib.sha256()
    for path in source_files():
        relative = path.relative_to(ROOT).as_posix().encode("utf-8")
        payload = path.read_bytes()
        hasher.update(len(relative).to_bytes(8, "big"))
        hasher.update(relative)
        hasher.update(len(payload).to_bytes(8, "big"))
        hasher.update(payload)
    return hasher.hexdigest()


def display_command(command: list[str]) -> list[str]:
    shown = list(command)
    if shown and Path(shown[0]).resolve() == Path(sys.executable).resolve():
        shown[0] = "python"
    return shown


def run_gate(
    name: str,
    command: list[str],
    *,
    logs: Path,
    tool: str | None = None,
) -> dict[str, Any]:
    shown = display_command(command)
    if tool is not None and shutil.which(tool) is None:
        return {
            "name": name,
            "status": "NOT_RUN",
            "command": shown,
            "reason": f"{tool} not found",
        }
    completed = subprocess.run(
        command,
        cwd=ROOT,
        env=ENV,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    log_path = logs / f"{name}.log"
    log_path.write_text(completed.stdout, encoding="utf-8")
    return {
        "name": name,
        "status": "PASS" if completed.returncode == 0 else "FAIL",
        "command": shown,
        "returncode": completed.returncode,
        "log": f"logs/{name}.log",
    }


def file_gate(name: str, path: Path, reason: str) -> dict[str, Any]:
    return {
        "name": name,
        "status": "PASS" if path.is_file() else "FAIL",
        "command": ["file-exists", str(path.relative_to(ROOT))],
        "reason": "present" if path.is_file() else reason,
    }


def equality_gate(name: str, before: str, after: str, reason: str) -> dict[str, Any]:
    return {
        "name": name,
        "status": "PASS" if before == after else "FAIL",
        "command": ["compare", "source-tree-fingerprint"],
        "reason": "identical" if before == after else reason,
    }


def write_reports(output: Path, gates: list[dict[str, Any]], epoch: int) -> str:
    freeze = "PASS" if all(gate["status"] == "PASS" for gate in gates) else "BLOCKED"
    source_tree_mutated = next(
        gate["status"] != "PASS" for gate in gates if gate["name"] == "source_tree_unchanged"
    )
    report = {
        "generated_at": datetime.fromtimestamp(epoch, UTC).isoformat(),
        "foundation": FOUNDATION,
        "freeze": freeze,
        "source_tree_mutated": source_tree_mutated,
        "gates": gates,
    }
    (output / "verification-results.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    lines = [
        "# Foundation Verification",
        "",
        "Generated outside the reproducible source archive by `scripts/run_verification.py`.",
        (
            "Source-tree fingerprint: **CHANGED**."
            if source_tree_mutated
            else "Source-tree fingerprint: **UNCHANGED**."
        ),
        "",
        f"- Foundation: **{FOUNDATION}**",
        f"- Freeze: **{freeze}**",
        "",
        "| Gate | Status | Command / reason |",
        "|---|---:|---|",
    ]
    for gate in gates:
        detail = gate.get("reason") or " `" + " ".join(gate["command"]) + "`"
        lines.append(f"| `{gate['name']}` | **{gate['status']}** | {detail} |")
    lines.extend(
        [
            "",
            "## Interpretation",
            "",
            "Every `NOT_RUN` or `FAIL` blocks V0.2.2 contract freeze."
            " M1 remains blocked until every required gate is `PASS`.",
            "The deterministic archive gate is deliberately last,"
            " and no archived file is modified afterward.",
            "",
        ]
    )
    (output / "FOUNDATION_VERIFICATION.md").write_text("\n".join(lines), encoding="utf-8")

    blockers = [gate for gate in gates if gate["status"] != "PASS"]
    blocker_lines = [
        "# Foundation Blockers",
        "",
        "Generated from the same gate result set as `FOUNDATION_VERIFICATION.md`.",
        "",
        "| Gate | Status | Reason / log |",
        "|---|---:|---|",
    ]
    for gate in blockers:
        detail = gate.get("reason") or gate.get("log") or "command failed"
        blocker_lines.append(f"| `{gate['name']}` | **{gate['status']}** | {detail} |")
    blocker_lines.extend(["", f"M1 unblocked: **{'yes' if freeze == 'PASS' else 'no'}**", ""])
    (output / "FOUNDATION_BLOCKERS.md").write_text("\n".join(blocker_lines), encoding="utf-8")

    status = {
        "version": "0.2.2",
        "foundation": FOUNDATION,
        "contract_closure_implemented": True,
        "playable_engine": False,
        "real_magic_rules": False,
        "real_card_support": False,
        "freeze": freeze,
        "m1_unblocked": freeze == "PASS",
        "source_tree_mutated": source_tree_mutated,
        "gate_status": {gate["name"]: gate["status"] for gate in gates},
    }
    (output / STATUS_FILENAME).write_text(
        json.dumps(status, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return freeze


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ROOT / "dist" / "verification",
        help="external directory for logs and generated reports",
    )
    args = parser.parse_args()
    source_before = source_tree_fingerprint()
    output = args.output_dir.resolve()
    if output == ROOT or (ROOT in output.parents and "dist" not in output.relative_to(ROOT).parts):
        parser.error("verification output must be outside the archived source set")
    if output.exists():
        marker = output / OUTPUT_MARKER
        if not marker.is_file():
            parser.error(
                "refusing to replace an existing unowned output directory; "
                f"expected marker {marker}"
            )
        shutil.rmtree(output)
    logs = output / "logs"
    logs.mkdir(parents=True, exist_ok=True)
    (output / OUTPUT_MARKER).write_text(
        "owned by scripts/run_verification.py\n",
        encoding="utf-8",
    )

    gates: list[dict[str, Any]] = [
        run_gate(
            "repository_verifier",
            [sys.executable, "scripts/verify_repository.py"],
            logs=logs,
        ),
        run_gate(
            "generated_contract_drift",
            [sys.executable, "scripts/generate_contracts.py", "--check"],
            logs=logs,
        ),
        run_gate(
            "synthetic_golden_path",
            [sys.executable, "scripts/validate_golden_path.py"],
            logs=logs,
        ),
        run_gate(
            "rust_source_structure",
            [sys.executable, "scripts/check_rust_source_structure.py"],
            logs=logs,
        ),
        run_gate(
            "documentation_contracts",
            [sys.executable, "scripts/check_documentation.py"],
            logs=logs,
        ),
        run_gate(
            "schema_validation",
            [sys.executable, "scripts/validate_schemas.py"],
            logs=logs,
        ),
        run_gate(
            "maintainer_artifacts",
            [sys.executable, "scripts/validate_maintainer_artifacts.py"],
            logs=logs,
        ),
        run_gate(
            "python_toolchain",
            [sys.executable, "scripts/verify_python_toolchain.py"],
            logs=logs,
        ),
        run_gate(
            "python_tests",
            [sys.executable, "scripts/run_python_tests.py"],
            logs=logs,
        ),
        run_gate(
            "ruff_format",
            ["ruff", "format", "--check", "python", "scripts"],
            logs=logs,
            tool="ruff",
        ),
        run_gate(
            "ruff",
            ["ruff", "check", "python", "scripts"],
            logs=logs,
            tool="ruff",
        ),
        run_gate(
            "mypy",
            ["mypy", "--config-file", "python/pyproject.toml"],
            logs=logs,
            tool="mypy",
        ),
        file_gate(
            "cargo_lock",
            ROOT / "Cargo.lock",
            "Cargo.lock must be generated and committed by the pinned Rust toolchain",
        ),
        run_gate(
            "cargo_fmt",
            ["cargo", "fmt", "--all", "--", "--check"],
            logs=logs,
            tool="cargo",
        ),
        run_gate(
            "cargo_check",
            [
                "cargo",
                "check",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--locked",
            ],
            logs=logs,
            tool="cargo",
        ),
        run_gate(
            "cargo_clippy",
            [
                "cargo",
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--locked",
                "--",
                "-D",
                "warnings",
            ],
            logs=logs,
            tool="cargo",
        ),
        run_gate(
            "cargo_test",
            ["cargo", "test", "--workspace", "--all-features", "--locked"],
            logs=logs,
            tool="cargo",
        ),
    ]

    source_after = source_tree_fingerprint()
    gates.append(
        equality_gate(
            "source_tree_unchanged",
            source_before,
            source_after,
            "a verification gate modified an archived source file",
        )
    )

    # This must remain the final gate. It observes the final source tree, and all
    # reports/logs live below dist/, which the archive builder excludes.
    gates.append(
        run_gate(
            "archive_reproducibility",
            [sys.executable, "scripts/verify_archive_reproducibility.py"],
            logs=logs,
        )
    )

    with (ROOT / "config" / "reproducibility.toml").open("rb") as handle:
        reproducibility = tomllib.load(handle)
    epoch = int(
        os.environ.get(
            "SOURCE_DATE_EPOCH",
            str(reproducibility["source_date_epoch"]),
        )
    )
    freeze = write_reports(output, gates, epoch)
    print(
        json.dumps(
            {
                "freeze": freeze,
                "output_dir": str(output),
                "gates": {gate["name"]: gate["status"] for gate in gates},
            },
            sort_keys=True,
        )
    )
    raise SystemExit(0 if freeze == "PASS" else 2)


if __name__ == "__main__":
    main()
