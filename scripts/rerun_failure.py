#!/usr/bin/env python3
"""Rerun one exact-head, trusted failure packet without mutating Git."""

from __future__ import annotations

import argparse
import shutil
import sys
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

sys.dont_write_bytecode = True

import failure_packet
from capture_failure import run_bounded

ROOT = failure_packet.ROOT
MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS = (
    failure_packet.MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS
)


@dataclass(frozen=True)
class RerunResult:
    status: str
    exit_code: int
    detail: str = ""


def _blocked(detail: str) -> RerunResult:
    return RerunResult(failure_packet.RERUN_BLOCKED, 2, detail)


def _not_reproduced(detail: str) -> RerunResult:
    return RerunResult(failure_packet.RERUN_NOT_REPRODUCED, 1, detail)


def _reproduced(detail: str) -> RerunResult:
    return RerunResult(failure_packet.RERUN_REPRODUCED, 0, detail)


def _source_identity_difference(
    recorded: dict[str, Any],
    actual: failure_packet.SourceIdentity,
) -> str | None:
    current = {
        "commit": actual.commit,
        "tree": actual.tree,
        "fingerprint": actual.fingerprint,
        "clean": actual.clean,
    }
    for field in ("commit", "tree", "fingerprint", "clean"):
        if recorded.get(field) != current[field]:
            return (
                f"source identity mismatch for {field}: "
                f"expected {recorded.get(field)!r}, got {current[field]!r}"
            )
    return None


def _tool_identity_difference(manifest: dict[str, Any]) -> str | None:
    recorded = manifest["tools"]["capture_python"]
    current = failure_packet.tool_identity()["capture_python"]
    if recorded["version"] != current["version"]:
        return (
            "required capture Python version differs: "
            f"expected {recorded['version']!r}, got {current['version']!r}"
        )
    return None


def _resolve_cwd(repository_root: Path, value: str) -> Path:
    cwd = (repository_root / value).resolve()
    if cwd != repository_root and repository_root not in cwd.parents:
        raise failure_packet.FailurePacketError("command.cwd escapes the repository")
    if not cwd.is_dir():
        raise failure_packet.FailurePacketError(f"command.cwd is not a directory: {cwd}")
    return cwd


def _executable_available(executable: str, cwd: Path) -> bool:
    path = Path(executable)
    if path.is_absolute() or "/" in executable or "\\" in executable:
        candidate = path if path.is_absolute() else cwd / path
        return candidate.is_file()
    return shutil.which(executable) is not None


def _validate_argv(argv: list[str], cwd: Path) -> str | None:
    if not argv:
        return "command argv is empty"
    if any(
        not isinstance(argument, str) or not argument or "\x00" in argument for argument in argv
    ):
        return "command argv must contain nonempty NUL-free strings"
    if not _executable_available(argv[0], cwd):
        return f"command executable is unavailable: {argv[0]}"
    return None


def _combined_log(outcome: failure_packet.CommandOutcome) -> bytes:
    return b"--- stdout ---\n" + outcome.stdout + b"\n--- stderr ---\n" + outcome.stderr


def _expected_marker(manifest: dict[str, Any]) -> dict[str, str] | None:
    signature = manifest["failure_signature"]
    fields = ("surface", "semantic_path", "mismatch_kind")
    if all(signature.get(field) is None for field in fields):
        return None
    return {field: signature[field] for field in fields}


def _marker_difference(
    manifest: dict[str, Any],
    outcome: failure_packet.CommandOutcome,
) -> str | None:
    try:
        actual = failure_packet.parse_signature_marker(_combined_log(outcome))
    except failure_packet.FailurePacketError as error:
        raise failure_packet.FailurePacketError(str(error)) from error
    expected = _expected_marker(manifest)
    if expected is None and actual is None:
        return None
    if expected is None:
        return "rerun emitted an unexpected structured failure signature"
    if actual is None:
        raise failure_packet.FailurePacketError(
            "rerun did not emit the required structured failure signature"
        )
    if actual != expected:
        return f"structured failure signature differs: expected {expected!r}, got {actual!r}"
    return None


def _predicate_result(
    manifest: dict[str, Any],
    outcome: failure_packet.CommandOutcome,
) -> RerunResult:
    execution = manifest["execution"]
    expected_outcome = execution["outcome"]
    if outcome.timed_out:
        observed_outcome = failure_packet.COMMAND_TIMEOUT
        observed_exit = None
    else:
        observed_outcome = failure_packet.COMMAND_EXIT
        observed_exit = outcome.returncode

    if expected_outcome == failure_packet.COMMAND_TIMEOUT:
        if observed_outcome != failure_packet.COMMAND_TIMEOUT:
            return _not_reproduced(
                "recorded COMMAND_TIMEOUT, but rerun produced an ordinary command exit"
            )
        return _reproduced("COMMAND_TIMEOUT predicate matched")

    if observed_outcome != failure_packet.COMMAND_EXIT:
        return _not_reproduced("recorded COMMAND_EXIT, but rerun reached the timeout condition")
    expected_exit = execution["exit_status"]
    if observed_exit != expected_exit:
        return _not_reproduced(
            f"exit status differs: expected {expected_exit!r}, got {observed_exit!r}"
        )
    return _reproduced("COMMAND_EXIT predicate matched")


def rerun(
    packet: Path,
    *,
    repository_root: Path = ROOT,
    source_identity_provider: Callable[[], failure_packet.SourceIdentity] | None = None,
) -> RerunResult:
    try:
        manifest = failure_packet.load_packet(Path(packet))
    except failure_packet.FailurePacketError as error:
        return _blocked(str(error))

    repository_root = Path(repository_root).resolve()
    if not repository_root.is_dir():
        return _blocked(f"repository root is not a directory: {repository_root}")

    provider = source_identity_provider or (
        lambda: failure_packet.read_source_identity(repository_root)
    )
    try:
        before = provider()
    except failure_packet.FailurePacketError as error:
        return _blocked(str(error))

    source_difference = _source_identity_difference(manifest["source"], before)
    if source_difference is not None:
        return _blocked(source_difference)

    tool_difference = _tool_identity_difference(manifest)
    if tool_difference is not None:
        return _blocked(tool_difference)

    command = manifest["command"]
    try:
        cwd = _resolve_cwd(repository_root, command["cwd"])
    except failure_packet.FailurePacketError as error:
        return _blocked(str(error))
    argv = list(command["argv"])
    command_error = _validate_argv(argv, cwd)
    if command_error is not None:
        return _blocked(command_error)

    outcome = run_bounded(
        argv,
        cwd=cwd,
        shell=False,
        timeout=manifest["execution"]["timeout_seconds"],
    )
    try:
        after = provider()
    except failure_packet.FailurePacketError as error:
        return _blocked(str(error))
    if before != after:
        return _blocked("rerun changed the source or Git identity")
    if not outcome.spawned:
        return _blocked("declared command could not be spawned")

    try:
        marker_difference = _marker_difference(manifest, outcome)
    except failure_packet.FailurePacketError as error:
        return _blocked(str(error))
    if marker_difference is not None:
        return _not_reproduced(marker_difference)
    return _predicate_result(manifest, outcome)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Rerun one exact-head trusted failure packet without Git mutation"
    )
    parser.add_argument("packet", type=Path)
    return parser


def main() -> int:
    args = _parser().parse_args()
    result = rerun(args.packet)
    print(result.status)
    if result.detail:
        print(result.detail)
    return result.exit_code


if __name__ == "__main__":
    raise SystemExit(main())
