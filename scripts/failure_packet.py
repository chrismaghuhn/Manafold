#!/usr/bin/env python3
"""Internal, trusted failure-packet primitives for maintainer tooling."""

from __future__ import annotations

import copy
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

sys.dont_write_bytecode = True

from run_verification import source_tree_fingerprint

ROOT = Path(__file__).resolve().parents[1]
GIT_METADATA_TIMEOUT_SECONDS = 30
FAILURE_PACKET_FORMAT = "manafold.failure-packet.v1"
MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS = 600
CAPTURE_PASS = "CAPTURE_PASS"
CAPTURE_COMMAND_EXIT = "CAPTURE_COMMAND_EXIT"
CAPTURE_TIMEOUT = "CAPTURE_TIMEOUT"
CAPTURE_BLOCKED = "CAPTURE_BLOCKED"
COMMAND_EXIT = "COMMAND_EXIT"
COMMAND_TIMEOUT = "COMMAND_TIMEOUT"
RERUN_REPRODUCED = "REPRODUCED"
RERUN_NOT_REPRODUCED = "NOT_REPRODUCED"
RERUN_BLOCKED = "BLOCKED"
OUTPUT_MARKER = ".mtgml-failure-output"

_HEX40_RE = re.compile(r"^[0-9a-f]{40}$")
_HEX64_RE = re.compile(r"^[0-9a-f]{64}$")
_PACKET_ID_RE = re.compile(r"^[a-z0-9][a-z0-9._-]*$")
_SIGNATURE_LINE_RE = re.compile(
    r"^MANAFOLD_FAILURE_SIGNATURE v1 "
    r"surface=(?P<surface>[a-z0-9_]+) "
    r"path=(?P<semantic_path>[A-Za-z0-9_.\[\]:-]+) "
    r"mismatch_kind=(?P<mismatch_kind>[a-z0-9_]+)$"
)
_ALLOWED_TOP_LEVEL = {
    "format",
    "packet_id",
    "sensitivity",
    "origin",
    "source",
    "command",
    "execution",
    "failure",
    "failure_signature",
    "artifacts",
    "tools",
    "reproduction",
}


class FailurePacketError(ValueError):
    """A failure packet is malformed, unsafe, or cannot be written."""


@dataclass(frozen=True)
class SourceIdentity:
    commit: str
    tree: str
    fingerprint: str
    clean: bool


@dataclass(frozen=True)
class CommandOutcome:
    returncode: int | None
    stdout: bytes = b""
    stderr: bytes = b""
    timed_out: bool = False
    spawned: bool = True


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def sha256_file(path: Path) -> str:
    try:
        return sha256_bytes(path.read_bytes())
    except OSError as error:
        raise FailurePacketError(f"cannot read packet artifact {path}: {error}") from error


def command_text(argv: list[str]) -> str:
    return subprocess.list2cmdline(argv)


def _git_probe(repository_root: Path, arguments: list[str]) -> str:
    command = ["git", *arguments]
    try:
        result = subprocess.run(
            command,
            cwd=repository_root,
            capture_output=True,
            text=True,
            check=False,
            shell=False,
            timeout=GIT_METADATA_TIMEOUT_SECONDS,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise FailurePacketError(
            f"Git metadata probe failed: {command_text(command)}: {error}"
        ) from error
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or "unknown Git error"
        raise FailurePacketError(f"Git metadata probe failed: {command_text(command)}: {detail}")
    return result.stdout.strip()


def read_source_identity(repository_root: Path = ROOT) -> SourceIdentity:
    repository_root = repository_root.resolve()
    commit = _git_probe(repository_root, ["rev-parse", "HEAD"])
    tree = _git_probe(repository_root, ["rev-parse", "HEAD^{tree}"])
    status = _git_probe(repository_root, ["status", "--porcelain", "--untracked-files=all"])
    try:
        fingerprint = source_tree_fingerprint()
    except (OSError, ValueError) as error:
        raise FailurePacketError(f"source fingerprint failed: {error}") from error
    if not _HEX40_RE.fullmatch(commit) or not _HEX40_RE.fullmatch(tree):
        raise FailurePacketError("Git returned a noncanonical commit or tree identity")
    if not _HEX64_RE.fullmatch(fingerprint):
        raise FailurePacketError("source fingerprint is not a lowercase SHA-256")
    return SourceIdentity(
        commit=commit,
        tree=tree,
        fingerprint=fingerprint,
        clean=status == "",
    )


def tool_identity() -> dict[str, dict[str, object]]:
    return {
        "capture_python": {
            "version": platform.python_version(),
            "required": True,
        }
    }


def _safe_relative_path(value: object, label: str) -> str:
    if not isinstance(value, str) or not value or "\x00" in value:
        raise FailurePacketError(f"invalid {label}")
    path = Path(value)
    if path.is_absolute() or ".." in path.parts or ":" in value:
        raise FailurePacketError(f"unsafe {label}: {value!r}")
    return value


def _safe_packet_id(value: object) -> str:
    if not isinstance(value, str) or _PACKET_ID_RE.fullmatch(value) is None:
        raise FailurePacketError(f"invalid packet_id: {value!r}")
    return value


def _require_mapping(value: object, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise FailurePacketError(f"{label} must be an object")
    return value


def _require_string(value: object, label: str) -> str:
    if not isinstance(value, str) or not value or "\x00" in value:
        raise FailurePacketError(f"{label} must be a nonempty string")
    return value


def _require_hex(value: object, label: str, pattern: re.Pattern[str]) -> str:
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        raise FailurePacketError(f"{label} must be lowercase hexadecimal")
    return value


def _copy_exact_mapping(
    value: object,
    label: str,
    allowed: set[str],
) -> dict[str, Any]:
    mapping = _require_mapping(value, label)
    unknown = set(mapping) - allowed
    if unknown:
        raise FailurePacketError(f"unsupported {label} fields: {sorted(unknown)}")
    return {key: copy.deepcopy(mapping[key]) for key in allowed if key in mapping}


def _copy_signature(value: object) -> dict[str, Any]:
    signature = _require_mapping(value, "failure_signature")
    allowed = {
        "origin_kind",
        "case_id",
        "failure_classification",
        "surface",
        "semantic_path",
        "mismatch_kind",
    }
    return {key: copy.deepcopy(signature[key]) for key in allowed if key in signature}


def _sanitized_manifest(value: object) -> dict[str, Any]:
    manifest = _require_mapping(value, "manifest")
    sanitized = {key: copy.deepcopy(manifest[key]) for key in _ALLOWED_TOP_LEVEL if key in manifest}
    sanitized["origin"] = _copy_exact_mapping(
        manifest.get("origin"),
        "origin",
        {"kind", "case_id"},
    )
    sanitized["source"] = _copy_exact_mapping(
        manifest.get("source"),
        "source",
        {"commit", "tree", "fingerprint", "clean"},
    )
    sanitized["command"] = _copy_exact_mapping(
        manifest.get("command"),
        "command",
        {"argv", "cwd"},
    )
    sanitized["execution"] = _copy_exact_mapping(
        manifest.get("execution"),
        "execution",
        {"outcome", "exit_status", "timeout_seconds"},
    )
    failure = _require_mapping(sanitized.get("failure"), "failure")
    sanitized["failure"] = {"classification": failure.get("classification")}
    tools = _require_mapping(sanitized.get("tools"), "tools")
    capture_python = _require_mapping(tools.get("capture_python"), "tools.capture_python")
    sanitized["tools"] = {
        "capture_python": {
            "version": capture_python.get("version"),
            "required": capture_python.get("required"),
        }
    }
    reproduction = _require_mapping(sanitized.get("reproduction"), "reproduction")
    sanitized["reproduction"] = {"display_command": reproduction.get("display_command")}
    sanitized["failure_signature"] = _copy_signature(sanitized.get("failure_signature"))
    return sanitized


def validate_manifest(value: object, *, require_artifact: bool = True) -> dict[str, Any]:
    manifest = _require_mapping(value, "manifest")
    unknown = set(manifest) - _ALLOWED_TOP_LEVEL
    if unknown:
        raise FailurePacketError(f"unsupported manifest fields: {sorted(unknown)}")
    if manifest.get("format") != FAILURE_PACKET_FORMAT:
        raise FailurePacketError(f"unsupported packet format: {manifest.get('format')!r}")
    packet_id = _safe_packet_id(manifest.get("packet_id"))
    sensitivity = manifest.get("sensitivity")
    if sensitivity not in {"public", "perspective_private", "trusted"}:
        raise FailurePacketError(f"unsupported packet sensitivity: {sensitivity!r}")

    origin = _require_mapping(manifest.get("origin"), "origin")
    _require_string(origin.get("kind"), "origin.kind")
    _require_string(origin.get("case_id"), "origin.case_id")

    source = _require_mapping(manifest.get("source"), "source")
    _require_hex(source.get("commit"), "source.commit", _HEX40_RE)
    _require_hex(source.get("tree"), "source.tree", _HEX40_RE)
    _require_hex(source.get("fingerprint"), "source.fingerprint", _HEX64_RE)
    if not isinstance(source.get("clean"), bool):
        raise FailurePacketError("source.clean must be boolean")

    command = _require_mapping(manifest.get("command"), "command")
    argv = command.get("argv")
    if not isinstance(argv, list) or not argv:
        raise FailurePacketError("command.argv must be a nonempty list")
    if any(
        not isinstance(argument, str) or not argument or "\x00" in argument for argument in argv
    ):
        raise FailurePacketError("command.argv entries must be nonempty NUL-free strings")
    _safe_relative_path(command.get("cwd"), "command.cwd")

    execution = _require_mapping(manifest.get("execution"), "execution")
    outcome = execution.get("outcome")
    if outcome not in {COMMAND_EXIT, COMMAND_TIMEOUT}:
        raise FailurePacketError(f"unsupported execution outcome: {outcome!r}")
    timeout_seconds = execution.get("timeout_seconds")
    if (
        isinstance(timeout_seconds, bool)
        or not isinstance(timeout_seconds, int)
        or timeout_seconds <= 0
        or timeout_seconds > MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS
    ):
        raise FailurePacketError("execution.timeout_seconds exceeds the hard limit")
    exit_status = execution.get("exit_status")
    if outcome == COMMAND_EXIT:
        if isinstance(exit_status, bool) or not isinstance(exit_status, int) or exit_status <= 0:
            raise FailurePacketError("COMMAND_EXIT requires a positive exit_status")
    elif exit_status is not None:
        raise FailurePacketError("COMMAND_TIMEOUT requires a null exit_status")

    failure = _require_mapping(manifest.get("failure"), "failure")
    if failure.get("classification") != outcome:
        raise FailurePacketError("failure.classification does not match execution.outcome")

    signature = _require_mapping(manifest.get("failure_signature"), "failure_signature")
    if signature.get("origin_kind") != origin.get("kind"):
        raise FailurePacketError("failure_signature.origin_kind does not match origin.kind")
    if signature.get("case_id") != origin.get("case_id"):
        raise FailurePacketError("failure_signature.case_id does not match origin.case_id")
    if signature.get("failure_classification") != outcome:
        raise FailurePacketError("failure_signature classification does not match outcome")
    marker_fields = ["surface", "semantic_path", "mismatch_kind"]
    present_marker_fields = [signature.get(field) is not None for field in marker_fields]
    if any(present_marker_fields) and not all(present_marker_fields):
        raise FailurePacketError("structured failure signature fields must be complete")
    for field in marker_fields:
        if signature.get(field) is not None:
            _require_string(signature[field], f"failure_signature.{field}")

    tools = _require_mapping(manifest.get("tools"), "tools")
    capture_python = _require_mapping(tools.get("capture_python"), "tools.capture_python")
    _require_string(capture_python.get("version"), "tools.capture_python.version")
    if capture_python.get("required") is not True:
        raise FailurePacketError("tools.capture_python.required must be true")

    reproduction = _require_mapping(manifest.get("reproduction"), "reproduction")
    _require_string(reproduction.get("display_command"), "reproduction.display_command")

    if require_artifact:
        artifacts = _require_mapping(manifest.get("artifacts"), "artifacts")
        log = _require_mapping(artifacts.get("log"), "artifacts.log")
        _safe_relative_path(log.get("path"), "artifacts.log.path")
        _require_hex(log.get("sha256"), "artifacts.log.sha256", _HEX64_RE)

    return {
        **manifest,
        "packet_id": packet_id,
    }


def parse_signature_marker(output: str | bytes) -> dict[str, str] | None:
    text = output.decode("utf-8", errors="replace") if isinstance(output, bytes) else output
    lines = [line for line in text.splitlines() if line.startswith("MANAFOLD_FAILURE_SIGNATURE")]
    if not lines:
        return None
    if len(lines) != 1:
        raise FailurePacketError("multiple failure signature markers were emitted")
    match = _SIGNATURE_LINE_RE.fullmatch(lines[0])
    if match is None:
        raise FailurePacketError("malformed failure signature marker")
    return match.groupdict()


def ensure_output_root(output_root: Path, repository_root: Path = ROOT) -> Path:
    candidate = Path(output_root)
    if ".." in candidate.parts:
        raise FailurePacketError(f"output root contains traversal: {output_root}")
    repository_root = Path(repository_root).resolve()
    if not candidate.is_absolute():
        candidate = repository_root / candidate
    resolved = candidate.resolve()
    if resolved == repository_root:
        raise FailurePacketError("output root cannot be the repository root")
    if repository_root in resolved.parents:
        relative = resolved.relative_to(repository_root)
        if not relative.parts or relative.parts[0].lower() != "dist":
            raise FailurePacketError("repository output must be under dist/")
    if resolved.exists():
        if not resolved.is_dir():
            raise FailurePacketError(f"output root is not a directory: {resolved}")
        if not (resolved / OUTPUT_MARKER).is_file():
            raise FailurePacketError(
                f"refusing unowned output root; expected marker {resolved / OUTPUT_MARKER}"
            )
    else:
        try:
            resolved.mkdir(parents=True, exist_ok=False)
            (resolved / OUTPUT_MARKER).write_text(
                "owned by internal Manafold failure-packet tooling\n",
                encoding="utf-8",
            )
        except OSError as error:
            raise FailurePacketError(f"cannot create output root {resolved}: {error}") from error
    return resolved


def _packet_artifact_path(packet: Path, relative: object) -> Path:
    relative_value = _safe_relative_path(relative, "packet artifact path")
    packet = packet.resolve()
    artifact = (packet / relative_value).resolve()
    if packet not in artifact.parents:
        raise FailurePacketError("packet artifact escapes packet directory")
    return artifact


def _json_bytes(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode("utf-8")


def load_packet(packet: Path) -> dict[str, Any]:
    packet = Path(packet)
    if not packet.is_dir():
        raise FailurePacketError(f"packet directory does not exist: {packet}")
    manifest_path = _packet_artifact_path(packet, "manifest.json")
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise FailurePacketError(f"cannot read packet manifest: {error}") from error
    validated = validate_manifest(manifest)
    log_path = _packet_artifact_path(packet, validated["artifacts"]["log"]["path"])
    if not log_path.is_file():
        raise FailurePacketError(f"packet log is missing: {log_path}")
    expected = validated["artifacts"]["log"]["sha256"]
    actual = sha256_file(log_path)
    if actual != expected:
        raise FailurePacketError(f"packet log checksum mismatch: expected {expected}, got {actual}")
    return validated


def write_packet(
    output_root: Path,
    packet_id: str,
    manifest: dict[str, Any],
    log: bytes,
    *,
    repository_root: Path = ROOT,
) -> Path:
    output = ensure_output_root(output_root, repository_root)
    safe_id = _safe_packet_id(packet_id)
    if not isinstance(log, bytes):
        raise FailurePacketError("packet log must be bytes")
    candidate = output / safe_id
    if candidate.exists():
        raise FailurePacketError(f"packet already exists: {candidate}")

    sanitized = _sanitized_manifest(manifest)
    if sanitized.get("packet_id") != safe_id:
        raise FailurePacketError("manifest packet_id does not match requested packet_id")
    log_checksum = sha256_bytes(log)
    sanitized["artifacts"] = {
        "log": {
            "path": "command.log",
            "sha256": log_checksum,
        }
    }
    validate_manifest(sanitized)

    stage: Path | None = None
    try:
        stage = Path(tempfile.mkdtemp(prefix=f".{safe_id}.", dir=str(output)))
        (stage / "command.log").write_bytes(log)
        (stage / "manifest.json").write_bytes(_json_bytes(sanitized))
        load_packet(stage)
        os.replace(str(stage), str(candidate))
        stage = None
    except (OSError, FailurePacketError) as error:
        if stage is not None:
            shutil.rmtree(stage, ignore_errors=True)
        raise FailurePacketError(f"cannot atomically write failure packet: {error}") from error
    return candidate


def safe_summary(manifest: dict[str, Any]) -> dict[str, Any]:
    """Construct a sink-safe summary from explicitly approved fields only."""

    source = _require_mapping(manifest.get("source"), "source")
    origin = _require_mapping(manifest.get("origin"), "origin")
    failure = _require_mapping(manifest.get("failure"), "failure")
    signature = _require_mapping(manifest.get("failure_signature"), "failure_signature")
    return {
        "format": manifest.get("format"),
        "packet_id": manifest.get("packet_id"),
        "origin": {
            "kind": origin.get("kind"),
            "case_id": origin.get("case_id"),
        },
        "failure": {
            "classification": failure.get("classification"),
        },
        "failure_signature": {
            "origin_kind": signature.get("origin_kind"),
            "case_id": signature.get("case_id"),
            "failure_classification": signature.get("failure_classification"),
            "surface": signature.get("surface"),
            "mismatch_kind": signature.get("mismatch_kind"),
        },
        "source": {
            "commit": source.get("commit"),
        },
    }
