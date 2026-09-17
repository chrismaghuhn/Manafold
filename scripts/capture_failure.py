#!/usr/bin/env python3
"""Capture one bounded, trusted command failure outside the source tree."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import uuid
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

sys.dont_write_bytecode = True

import failure_packet

ROOT = failure_packet.ROOT
MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS = (
    failure_packet.MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS
)


@dataclass(frozen=True)
class CaptureResult:
    status: str
    exit_code: int
    packet: Path | None = None
    detail: str = ""

    @property
    def rerun_command(self) -> str | None:
        if self.packet is None:
            return None
        return failure_packet.command_text(
            ["<project-python>", "scripts/rerun_failure.py", str(self.packet)]
        )


def _as_bytes(value: bytes | str | None) -> bytes:
    if value is None:
        return b""
    return value if isinstance(value, bytes) else value.encode("utf-8", errors="replace")


def _executable_available(executable: str) -> bool:
    path = Path(executable)
    if path.is_absolute() or "/" in executable or "\\" in executable:
        return path.is_file()
    return shutil.which(executable) is not None


def _validate_argv(argv: list[str]) -> str | None:
    if not argv:
        return "command argv is empty"
    if any(
        not isinstance(argument, str) or not argument or "\x00" in argument for argument in argv
    ):
        return "command argv must contain nonempty NUL-free strings"
    if not _executable_available(argv[0]):
        return f"command executable is unavailable: {argv[0]}"
    return None


def run_bounded(
    argv: list[str],
    *,
    cwd: Path,
    shell: bool = False,
    timeout: int = MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS,
) -> failure_packet.CommandOutcome:
    try:
        completed = subprocess.run(
            argv,
            cwd=cwd,
            capture_output=True,
            text=False,
            check=False,
            shell=shell,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as error:
        return failure_packet.CommandOutcome(
            returncode=None,
            stdout=_as_bytes(error.stdout),
            stderr=_as_bytes(error.stderr),
            timed_out=True,
            spawned=True,
        )
    except (FileNotFoundError, OSError):
        return failure_packet.CommandOutcome(returncode=None, spawned=False)
    return failure_packet.CommandOutcome(
        returncode=completed.returncode,
        stdout=_as_bytes(completed.stdout),
        stderr=_as_bytes(completed.stderr),
    )


def _blocked(detail: str) -> CaptureResult:
    return CaptureResult(failure_packet.CAPTURE_BLOCKED, 2, detail=detail)


def _source_unchanged(
    before: failure_packet.SourceIdentity,
    after: failure_packet.SourceIdentity,
) -> bool:
    return before == after and before.clean and after.clean


def _manifest(
    *,
    case_id: str,
    argv: list[str],
    source: failure_packet.SourceIdentity,
    outcome: str,
    exit_status: int | None,
    marker: dict[str, str] | None,
) -> dict[str, Any]:
    signature = {
        "origin_kind": "command",
        "case_id": case_id,
        "failure_classification": outcome,
        "surface": marker["surface"] if marker else None,
        "semantic_path": marker["semantic_path"] if marker else None,
        "mismatch_kind": marker["mismatch_kind"] if marker else None,
    }
    return {
        "format": failure_packet.FAILURE_PACKET_FORMAT,
        "packet_id": "pending",
        "sensitivity": "trusted",
        "origin": {"kind": "command", "case_id": case_id},
        "source": {
            "commit": source.commit,
            "tree": source.tree,
            "fingerprint": source.fingerprint,
            "clean": source.clean,
        },
        "command": {"argv": list(argv), "cwd": "."},
        "execution": {
            "outcome": outcome,
            "exit_status": exit_status,
            "timeout_seconds": MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS,
        },
        "failure": {"classification": outcome},
        "failure_signature": signature,
        "tools": failure_packet.tool_identity(),
        "reproduction": {
            "display_command": "<project-python> scripts/rerun_failure.py <packet>",
        },
    }


def capture(
    argv: list[str],
    *,
    case_id: str,
    output_root: Path,
    repository_root: Path = ROOT,
    source_identity_provider: Callable[[], failure_packet.SourceIdentity] | None = None,
) -> CaptureResult:
    try:
        failure_packet.validate_case_id(case_id)
    except failure_packet.FailurePacketError as error:
        return _blocked(str(error))
    command_error = _validate_argv(argv)
    if command_error is not None:
        return _blocked(command_error)
    repository_root = Path(repository_root).resolve()
    if not repository_root.is_dir():
        return _blocked(f"repository root is not a directory: {repository_root}")
    try:
        failure_packet.validate_output_root(output_root, repository_root)
    except failure_packet.FailurePacketError as error:
        return _blocked(str(error))

    provider = source_identity_provider or (
        lambda: failure_packet.read_source_identity(repository_root)
    )
    try:
        before = provider()
    except failure_packet.FailurePacketError as error:
        return _blocked(str(error))
    if not before.clean:
        return _blocked("capture requires a clean source baseline")

    outcome = run_bounded(
        argv,
        cwd=repository_root,
        shell=False,
        timeout=MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS,
    )
    try:
        after = provider()
    except failure_packet.FailurePacketError as error:
        return _blocked(str(error))
    if not _source_unchanged(before, after):
        return _blocked("command changed the source or Git identity")
    if not outcome.spawned:
        return _blocked("declared command could not be spawned")

    log = b"--- stdout ---\n" + outcome.stdout + b"\n--- stderr ---\n" + outcome.stderr
    try:
        marker = failure_packet.parse_signature_marker(log)
    except failure_packet.FailurePacketError as error:
        return _blocked(str(error))
    try:
        context = failure_packet.parse_t0_failure_context(log)
    except failure_packet.FailurePacketError as error:
        return _blocked(str(error))
    if context is not None and context["case"] != case_id:
        return _blocked(
            "declared case id does not match the T0 failure context "
            f"(declared {case_id!r}, context {context['case']!r})"
        )
    if context is not None:
        # A T0 context without a structured failure signature, or one that
        # disagrees with it, is incoherent evidence: the context claims a
        # specific first divergence that the signature must confirm.
        if marker is None:
            return _blocked("T0 failure context requires a structured failure signature")
        for context_field, marker_field in (
            ("surface", "surface"),
            ("path", "semantic_path"),
            ("kind", "mismatch_kind"),
        ):
            if context[context_field] != marker[marker_field]:
                return _blocked(
                    "T0 failure context disagrees with the failure signature "
                    f"({context_field} {context[context_field]!r} != "
                    f"{marker_field} {marker[marker_field]!r})"
                )

    if outcome.returncode == 0 and not outcome.timed_out:
        return CaptureResult(failure_packet.CAPTURE_PASS, 0)
    if outcome.timed_out:
        outcome_kind = failure_packet.COMMAND_TIMEOUT
        exit_status = None
        capture_status = failure_packet.CAPTURE_TIMEOUT
        exit_code = 124
    elif outcome.returncode is not None and outcome.returncode > 0:
        outcome_kind = failure_packet.COMMAND_EXIT
        exit_status = outcome.returncode
        capture_status = failure_packet.CAPTURE_COMMAND_EXIT
        exit_code = outcome.returncode
    else:
        return _blocked("command returned an unsupported nonzero status")

    packet_id = uuid.uuid4().hex
    manifest = _manifest(
        case_id=case_id,
        argv=argv,
        source=before,
        outcome=outcome_kind,
        exit_status=exit_status,
        marker=marker,
    )
    manifest["packet_id"] = packet_id
    try:
        packet_path = failure_packet.write_packet(
            output_root,
            packet_id,
            manifest,
            log,
            repository_root=repository_root,
        )
    except failure_packet.FailurePacketError as error:
        return _blocked(str(error))
    return CaptureResult(capture_status, exit_code, packet=packet_path)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Capture one bounded command failure in a trusted local packet"
    )
    parser.add_argument("--case-id", required=True)
    parser.add_argument(
        "--output-root",
        type=Path,
        default=ROOT / "dist" / "failures",
    )
    parser.add_argument("command", nargs=argparse.REMAINDER)
    return parser


def main() -> int:
    args = _parser().parse_args()
    argv = list(args.command)
    if argv[:1] == ["--"]:
        argv = argv[1:]
    if not argv:
        _parser().error("a command is required after --")
    result = capture(
        argv,
        case_id=args.case_id,
        output_root=args.output_root,
    )
    print(result.status)
    if result.packet is not None:
        print(f"PACKET: {result.packet}")
        print(f"RERUN: {result.rerun_command}")
    elif result.detail:
        print(f"{result.status}: {result.detail}", file=sys.stderr)
    return result.exit_code


if __name__ == "__main__":
    raise SystemExit(main())
