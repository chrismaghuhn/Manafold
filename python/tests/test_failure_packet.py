from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import failure_packet
import run_verification


def base_manifest() -> dict[str, Any]:
    return {
        "format": failure_packet.FAILURE_PACKET_FORMAT,
        "packet_id": "case-1",
        "sensitivity": "trusted",
        "origin": {"kind": "command", "case_id": "CASE_1"},
        "source": {
            "commit": "a" * 40,
            "tree": "b" * 40,
            "fingerprint": "c" * 64,
            "clean": True,
        },
        "command": {
            "argv": ["python", "-c", "raise SystemExit(1)"],
            "cwd": ".",
        },
        "execution": {
            "outcome": failure_packet.COMMAND_EXIT,
            "exit_status": 1,
            "timeout_seconds": failure_packet.MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS,
        },
        "failure": {"classification": failure_packet.COMMAND_EXIT},
        "failure_signature": {
            "origin_kind": "command",
            "case_id": "CASE_1",
            "failure_classification": failure_packet.COMMAND_EXIT,
            "surface": None,
            "semantic_path": None,
            "mismatch_kind": None,
        },
        "tools": {
            "capture_python": {"version": "3.13.15", "required": True},
        },
        "reproduction": {
            "display_command": "<project-python> scripts/rerun_failure.py <packet>",
        },
    }


class FailurePacketCoreTests(unittest.TestCase):
    def test_packet_write_and_validate_round_trip(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output_root = Path(temporary) / "output"
            packet = failure_packet.write_packet(
                output_root,
                "case-1",
                base_manifest(),
                b"synthetic failure\n",
            )

            loaded = failure_packet.load_packet(packet)

            self.assertTrue(packet.joinpath("manifest.json").is_file())
            self.assertTrue(packet.joinpath("command.log").is_file())

        self.assertEqual(loaded["format"], failure_packet.FAILURE_PACKET_FORMAT)
        self.assertEqual(loaded["packet_id"], "case-1")
        self.assertEqual(loaded["artifacts"]["log"]["path"], "command.log")

    def test_packet_records_commit_tree_and_fingerprint_separately(self) -> None:
        manifest = base_manifest()
        with tempfile.TemporaryDirectory() as temporary:
            packet = failure_packet.write_packet(
                Path(temporary) / "output",
                "case-1",
                manifest,
                b"failure\n",
            )
            loaded = failure_packet.load_packet(packet)

        source = loaded["source"]
        self.assertEqual(source["commit"], "a" * 40)
        self.assertEqual(source["tree"], "b" * 40)
        self.assertEqual(source["fingerprint"], "c" * 64)
        self.assertNotEqual(source["commit"], source["tree"])
        self.assertNotEqual(source["tree"], source["fingerprint"])

    def test_packet_preserves_argv_as_a_list(self) -> None:
        manifest = base_manifest()
        manifest["command"]["argv"] = ["tool.exe", "--case", "value with spaces"]
        with tempfile.TemporaryDirectory() as temporary:
            packet = failure_packet.write_packet(
                Path(temporary) / "output",
                "case-1",
                manifest,
                b"failure\n",
            )
            loaded = failure_packet.load_packet(packet)

        self.assertEqual(
            loaded["command"]["argv"],
            ["tool.exe", "--case", "value with spaces"],
        )

    def test_packet_output_can_live_outside_source_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository_root = Path(temporary) / "repo"
            output_root = Path(temporary) / "external-output"
            repository_root.mkdir()
            packet = failure_packet.write_packet(
                output_root,
                "case-1",
                base_manifest(),
                b"failure\n",
                repository_root=repository_root,
            )

        self.assertFalse(packet.is_relative_to(repository_root))

    def test_packet_log_checksum_verifies(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            packet = failure_packet.write_packet(
                Path(temporary) / "output",
                "case-1",
                base_manifest(),
                b"failure\n",
            )
            loaded = failure_packet.load_packet(packet)
            checksum = loaded["artifacts"]["log"]["sha256"]
            self.assertEqual(checksum, failure_packet.sha256_file(packet / "command.log"))

        self.assertEqual(len(checksum), 64)

    def test_corrupt_log_checksum_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            packet = failure_packet.write_packet(
                Path(temporary) / "output",
                "case-1",
                base_manifest(),
                b"failure\n",
            )
            (packet / "command.log").write_bytes(b"corrupted\n")

            with self.assertRaises(failure_packet.FailurePacketError):
                failure_packet.load_packet(packet)

    def test_unsupported_packet_version_is_rejected_before_final_write(self) -> None:
        manifest = base_manifest()
        manifest["format"] = "manafold.failure-packet.v999"
        with tempfile.TemporaryDirectory() as temporary:
            output_root = Path(temporary) / "output"
            with self.assertRaises(failure_packet.FailurePacketError):
                failure_packet.write_packet(output_root, "case-1", manifest, b"failure\n")
            self.assertFalse((output_root / "case-1").exists())

    def test_invalid_output_root_and_traversal_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository_root = Path(temporary) / "repo"
            repository_root.mkdir()
            with self.assertRaises(failure_packet.FailurePacketError):
                failure_packet.ensure_output_root(repository_root, repository_root)
            with self.assertRaises(failure_packet.FailurePacketError):
                failure_packet.ensure_output_root(
                    repository_root / ".." / "outside",
                    repository_root,
                )

            unowned = repository_root / "dist" / "failures"
            unowned.mkdir(parents=True)
            (unowned / "precious.txt").write_text("keep\n", encoding="utf-8")
            with self.assertRaises(failure_packet.FailurePacketError):
                failure_packet.ensure_output_root(unowned, repository_root)
            self.assertEqual((unowned / "precious.txt").read_text(encoding="utf-8"), "keep\n")

    def test_repository_relative_output_root_resolves_from_repository_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository_root = Path(temporary) / "repo"
            repository_root.mkdir()

            resolved = failure_packet.ensure_output_root(
                Path("dist") / "failures",
                repository_root,
            )

        self.assertEqual(resolved, (repository_root / "dist" / "failures").resolve())

    def test_atomic_write_does_not_leave_partial_packet(self) -> None:
        manifest = base_manifest()
        manifest["command"]["argv"] = []
        with tempfile.TemporaryDirectory() as temporary:
            output_root = Path(temporary) / "output"
            with self.assertRaises(failure_packet.FailurePacketError):
                failure_packet.write_packet(output_root, "case-1", manifest, b"failure\n")
            self.assertFalse((output_root / "case-1").exists())
            self.assertEqual(
                [path for path in output_root.iterdir() if path.name.startswith(".case-1")],
                [],
            )

    def test_signature_marker_returns_only_stable_fields(self) -> None:
        marker = (
            "MANAFOLD_FAILURE_SIGNATURE v1 "
            "surface=events path=transition.events[3] mismatch_kind=value_changed"
        )
        self.assertEqual(
            failure_packet.parse_signature_marker(marker),
            {
                "surface": "events",
                "semantic_path": "transition.events[3]",
                "mismatch_kind": "value_changed",
            },
        )
        with self.assertRaises(failure_packet.FailurePacketError):
            failure_packet.parse_signature_marker(marker + "\n" + marker)
        with self.assertRaises(failure_packet.FailurePacketError):
            failure_packet.parse_signature_marker("MANAFOLD_FAILURE_SIGNATURE v1 malformed")

    def test_safe_summary_omits_trusted_and_secret_sentinels(self) -> None:
        manifest = base_manifest()
        manifest["command"]["argv"] = ["ROOT_SEED_SECRET_SENTINEL"]
        manifest["command"]["cwd"] = "PRIVATE_HAND_SENTINEL"
        manifest["tools"]["capture_python"]["version"] = "ROOT_SEED_SECRET_SENTINEL"
        manifest["reproduction"]["display_command"] = "PRIVATE_HAND_SENTINEL"
        manifest["failure_signature"]["surface"] = "events"
        manifest["failure_signature"]["semantic_path"] = (
            "zones.locations[object:INTERNAL_OBJECT_ID_SENTINEL]"
        )
        manifest["failure_signature"]["mismatch_kind"] = "value_changed"

        summary = failure_packet.safe_summary(manifest)
        rendered = json.dumps(summary, sort_keys=True)

        self.assertNotIn("ROOT_SEED_SECRET_SENTINEL", rendered)
        self.assertNotIn("INTERNAL_OBJECT_ID_SENTINEL", rendered)
        self.assertNotIn("PRIVATE_HAND_SENTINEL", rendered)
        self.assertEqual(summary["failure"]["classification"], "COMMAND_EXIT")

    def test_packet_contract_rejects_nontrusted_or_unsupported_identity(self) -> None:
        cases = {
            "public sensitivity": {"sensitivity": "public"},
            "perspective-private sensitivity": {"sensitivity": "perspective_private"},
            "non-command origin": {
                "origin": {"kind": "root_seed_secret_sentinel", "case_id": "CASE_1"}
            },
            "unsafe case ID": {"origin": {"kind": "command", "case_id": "CASE WITH SPACE"}},
        }
        for label, changes in cases.items():
            with self.subTest(label=label):
                manifest = base_manifest()
                for section, values in changes.items():
                    if isinstance(values, dict):
                        manifest[section].update(values)
                    else:
                        manifest[section] = values
                with self.assertRaises(failure_packet.FailurePacketError):
                    failure_packet.validate_manifest(manifest, require_artifact=False)

    def test_packet_contract_rejects_unknown_structured_tokens(self) -> None:
        for field, value in (
            ("surface", "ROOT_SEED_SECRET_SENTINEL"),
            ("mismatch_kind", "INTERNAL_OBJECT_ID_SENTINEL"),
        ):
            with self.subTest(field=field):
                manifest = base_manifest()
                manifest["failure_signature"].update(
                    {
                        "surface": "events",
                        "semantic_path": "transition.events[3]",
                        "mismatch_kind": "value_changed",
                        field: value,
                    }
                )
                with self.assertRaises(failure_packet.FailurePacketError):
                    failure_packet.validate_manifest(manifest, require_artifact=False)

    def test_safe_summary_fails_closed_for_unvalidated_signature_tokens(self) -> None:
        manifest = base_manifest()
        manifest["failure_signature"].update(
            {
                "surface": "ROOT_SEED_SECRET_SENTINEL",
                "semantic_path": "transition.events[3]",
                "mismatch_kind": "value_changed",
            }
        )
        with self.assertRaises(failure_packet.FailurePacketError):
            failure_packet.safe_summary(manifest)

    def test_nested_unapproved_fields_are_rejected(self) -> None:
        nested_fields = {
            "origin": {"internal_id": "INTERNAL_OBJECT_ID_SENTINEL"},
            "source": {"root_seed": "ROOT_SEED_SECRET_SENTINEL"},
            "command": {"environment": {"TOKEN": "SECRET_ENV_SENTINEL"}},
            "execution": {"private_detail": "PRIVATE_HAND_SENTINEL"},
        }
        for section, extras in nested_fields.items():
            with self.subTest(section=section):
                manifest = base_manifest()
                manifest[section].update(extras)
                with (
                    tempfile.TemporaryDirectory() as temporary,
                    self.assertRaises(failure_packet.FailurePacketError),
                ):
                    failure_packet.write_packet(
                        Path(temporary) / "output",
                        "case-1",
                        manifest,
                        b"failure\n",
                    )

    def test_load_packet_rejects_nested_unapproved_fields(self) -> None:
        nested_fields = {
            "origin": {"internal_id": "INTERNAL_OBJECT_ID_SENTINEL"},
            "source": {"root_seed": "ROOT_SEED_SECRET_SENTINEL"},
            "command": {"environment": {"TOKEN": "SECRET_ENV_SENTINEL"}},
            "execution": {"private_detail": "PRIVATE_HAND_SENTINEL"},
        }
        with tempfile.TemporaryDirectory() as temporary:
            packet = failure_packet.write_packet(
                Path(temporary) / "output",
                "case-1",
                base_manifest(),
                b"failure\n",
            )
            manifest_path = packet / "manifest.json"
            original = manifest_path.read_text(encoding="utf-8")
            for section, extras in nested_fields.items():
                with self.subTest(section=section):
                    manifest = json.loads(original)
                    manifest[section].update(extras)
                    manifest_path.write_text(
                        json.dumps(manifest),
                        encoding="utf-8",
                    )
                    with self.assertRaises(failure_packet.FailurePacketError):
                        failure_packet.load_packet(packet)
                    manifest_path.write_text(original, encoding="utf-8")

    def test_packet_does_not_snapshot_process_environment(self) -> None:
        manifest = base_manifest()
        manifest["environment"] = {"SECRET_ENV_SENTINEL": "must-not-be-captured"}
        with tempfile.TemporaryDirectory() as temporary:
            packet = failure_packet.write_packet(
                Path(temporary) / "output",
                "case-1",
                manifest,
                b"failure\n",
            )
            loaded = failure_packet.load_packet(packet)

        self.assertNotIn("environment", loaded)

    def test_source_fingerprint_reuses_run_verification_implementation(self) -> None:
        self.assertEqual(
            failure_packet.source_tree_fingerprint(),
            run_verification.source_tree_fingerprint(),
        )

    def test_failure_reproducer_scripts_are_repository_guarded(self) -> None:
        verifier = (ROOT / "scripts" / "verify_repository.py").read_text(encoding="utf-8")
        for script in (
            "scripts/failure_packet.py",
            "scripts/capture_failure.py",
            "scripts/rerun_failure.py",
        ):
            with self.subTest(script=script):
                self.assertIn(f'"{script}"', verifier)

    def test_run_checks_keeps_failure_reproducer_opt_in(self) -> None:
        run_checks = (ROOT / "scripts" / "run_checks.py").read_text(encoding="utf-8")
        self.assertNotIn("capture_failure.py", run_checks)
        self.assertNotIn("rerun_failure.py", run_checks)


class CaptureFailureTests(unittest.TestCase):
    @staticmethod
    def repository_root(temporary: str) -> Path:
        repository_root = Path(temporary) / "repo"
        repository_root.mkdir()
        return repository_root

    @staticmethod
    def identity_provider(
        *,
        fingerprint: str = "c" * 64,
        clean: bool = True,
    ) -> failure_packet.SourceIdentity:
        return failure_packet.SourceIdentity(
            commit="a" * 40,
            tree="b" * 40,
            fingerprint=fingerprint,
            clean=clean,
        )

    @staticmethod
    def failing_command(exit_status: int = 1) -> list[str]:
        return [
            sys.executable,
            "-c",
            f"import sys; sys.exit({exit_status})",
        ]

    def test_failing_command_creates_packet_and_preserves_exit(self) -> None:
        import capture_failure

        with tempfile.TemporaryDirectory() as temporary:
            output_root = Path(temporary) / "output"
            result = capture_failure.capture(
                self.failing_command(7),
                case_id="CAPTURE_FAIL",
                output_root=output_root,
                repository_root=self.repository_root(temporary),
                source_identity_provider=self.identity_provider,
            )

            self.assertEqual(result.status, failure_packet.CAPTURE_COMMAND_EXIT)
            self.assertEqual(result.exit_code, 7)
            self.assertIsNotNone(result.packet)
            self.assertTrue(result.packet.joinpath("manifest.json").is_file())
            manifest = failure_packet.load_packet(result.packet)

        self.assertEqual(manifest["execution"]["outcome"], failure_packet.COMMAND_EXIT)
        self.assertEqual(manifest["execution"]["exit_status"], 7)

    def test_passing_command_creates_no_packet(self) -> None:
        import capture_failure

        with tempfile.TemporaryDirectory() as temporary:
            output_root = Path(temporary) / "output"
            result = capture_failure.capture(
                self.failing_command(0),
                case_id="CAPTURE_PASS",
                output_root=output_root,
                repository_root=self.repository_root(temporary),
                source_identity_provider=self.identity_provider,
            )

            self.assertEqual(result.status, failure_packet.CAPTURE_PASS)
            self.assertEqual(result.exit_code, 0)
            self.assertIsNone(result.packet)
            self.assertFalse(output_root.exists())

    def test_unowned_output_root_blocks_before_failing_command(self) -> None:
        import capture_failure

        with tempfile.TemporaryDirectory() as temporary:
            output_root = Path(temporary) / "unowned"
            output_root.mkdir()
            output_root.joinpath("precious.txt").write_text("keep\n", encoding="utf-8")
            provider = mock.Mock(return_value=self.identity_provider())
            with mock.patch.object(
                capture_failure,
                "run_bounded",
                return_value=failure_packet.CommandOutcome(returncode=1),
            ) as run_bounded:
                result = capture_failure.capture(
                    self.failing_command(1),
                    case_id="CAPTURE_UNOWNED_FAIL",
                    output_root=output_root,
                    repository_root=self.repository_root(temporary),
                    source_identity_provider=provider,
                )
            self.assertTrue(output_root.joinpath("precious.txt").is_file())

        self.assertEqual(result.status, failure_packet.CAPTURE_BLOCKED)
        self.assertEqual(result.exit_code, 2)
        self.assertIsNone(result.packet)
        run_bounded.assert_not_called()
        provider.assert_not_called()

    def test_unowned_output_root_blocks_before_passing_command(self) -> None:
        import capture_failure

        with tempfile.TemporaryDirectory() as temporary:
            output_root = Path(temporary) / "unowned"
            output_root.mkdir()
            output_root.joinpath("precious.txt").write_text("keep\n", encoding="utf-8")
            provider = mock.Mock(return_value=self.identity_provider())
            with mock.patch.object(
                capture_failure,
                "run_bounded",
                return_value=failure_packet.CommandOutcome(returncode=0),
            ) as run_bounded:
                result = capture_failure.capture(
                    self.failing_command(0),
                    case_id="CAPTURE_UNOWNED_PASS",
                    output_root=output_root,
                    repository_root=self.repository_root(temporary),
                    source_identity_provider=provider,
                )
            self.assertTrue(output_root.joinpath("precious.txt").is_file())

        self.assertEqual(result.status, failure_packet.CAPTURE_BLOCKED)
        self.assertEqual(result.exit_code, 2)
        self.assertIsNone(result.packet)
        run_bounded.assert_not_called()
        provider.assert_not_called()

    def test_missing_command_is_blocked_without_packet(self) -> None:
        import capture_failure

        with tempfile.TemporaryDirectory() as temporary:
            output_root = Path(temporary) / "output"
            result = capture_failure.capture(
                [str(Path(temporary) / "missing-command.exe")],
                case_id="CAPTURE_MISSING",
                output_root=output_root,
                repository_root=self.repository_root(temporary),
                source_identity_provider=self.identity_provider,
            )

        self.assertEqual(result.status, failure_packet.CAPTURE_BLOCKED)
        self.assertEqual(result.exit_code, 2)
        self.assertIsNone(result.packet)

    def test_timeout_creates_distinct_timeout_packet(self) -> None:
        import capture_failure

        timeout = failure_packet.CommandOutcome(
            returncode=None,
            timed_out=True,
            stdout=b"partial stdout\n",
            stderr=b"partial stderr\n",
        )
        with tempfile.TemporaryDirectory() as temporary:
            output_root = Path(temporary) / "output"
            with mock.patch.object(capture_failure, "run_bounded", return_value=timeout):
                result = capture_failure.capture(
                    self.failing_command(),
                    case_id="CAPTURE_TIMEOUT",
                    output_root=output_root,
                    repository_root=self.repository_root(temporary),
                    source_identity_provider=self.identity_provider,
                )

            self.assertEqual(result.status, failure_packet.CAPTURE_TIMEOUT)
            self.assertEqual(result.exit_code, 124)
            manifest = failure_packet.load_packet(result.packet)

        self.assertEqual(manifest["execution"]["outcome"], failure_packet.COMMAND_TIMEOUT)
        self.assertIsNone(manifest["execution"]["exit_status"])

    def test_source_mutation_is_blocked_without_packet(self) -> None:
        import capture_failure

        snapshots = iter(
            [
                self.identity_provider(),
                self.identity_provider(fingerprint="d" * 64),
            ]
        )
        with tempfile.TemporaryDirectory() as temporary:
            output_root = Path(temporary) / "output"
            result = capture_failure.capture(
                self.failing_command(),
                case_id="CAPTURE_MUTATION",
                output_root=output_root,
                repository_root=self.repository_root(temporary),
                source_identity_provider=lambda: next(snapshots),
            )

        self.assertEqual(result.status, failure_packet.CAPTURE_BLOCKED)
        self.assertEqual(result.exit_code, 2)
        self.assertIsNone(result.packet)

    def test_malformed_marker_is_blocked_without_packet(self) -> None:
        import capture_failure

        command = [
            sys.executable,
            "-c",
            "print('MANAFOLD_FAILURE_SIGNATURE v1 malformed'); raise SystemExit(1)",
        ]
        with tempfile.TemporaryDirectory() as temporary:
            output_root = Path(temporary) / "output"
            result = capture_failure.capture(
                command,
                case_id="CAPTURE_MARKER",
                output_root=output_root,
                repository_root=self.repository_root(temporary),
                source_identity_provider=self.identity_provider,
            )

        self.assertEqual(result.status, failure_packet.CAPTURE_BLOCKED)
        self.assertEqual(result.exit_code, 2)
        self.assertIsNone(result.packet)

    def test_valid_marker_is_stored_as_structured_signature(self) -> None:
        import capture_failure

        command = [
            sys.executable,
            "-c",
            (
                "print('MANAFOLD_FAILURE_SIGNATURE v1 "
                "surface=events path=transition.events[3] "
                "mismatch_kind=value_changed'); raise SystemExit(1)"
            ),
        ]
        with tempfile.TemporaryDirectory() as temporary:
            result = capture_failure.capture(
                command,
                case_id="CAPTURE_MARKER",
                output_root=Path(temporary) / "output",
                repository_root=self.repository_root(temporary),
                source_identity_provider=self.identity_provider,
            )
            manifest = failure_packet.load_packet(result.packet)

        self.assertEqual(
            manifest["failure_signature"]["semantic_path"],
            "transition.events[3]",
        )

    def test_capture_uses_shell_false_and_hard_timeout(self) -> None:
        import capture_failure

        outcome = failure_packet.CommandOutcome(returncode=1)
        with tempfile.TemporaryDirectory() as temporary:
            repository_root = self.repository_root(temporary)
            output_root = Path(temporary) / "output"
            with mock.patch.object(
                capture_failure,
                "run_bounded",
                return_value=outcome,
            ) as run_bounded:
                result = capture_failure.capture(
                    self.failing_command(),
                    case_id="CAPTURE_BOUND",
                    output_root=output_root,
                    repository_root=repository_root,
                    source_identity_provider=self.identity_provider,
                )

        self.assertEqual(result.status, failure_packet.CAPTURE_COMMAND_EXIT)
        run_bounded.assert_called_once()
        self.assertFalse(run_bounded.call_args.kwargs["shell"])
        self.assertLessEqual(
            run_bounded.call_args.kwargs["timeout"],
            failure_packet.MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS,
        )

    def test_capture_source_identity_is_unchanged_on_successful_capture(self) -> None:
        import capture_failure

        snapshots = iter([self.identity_provider(), self.identity_provider()])
        with tempfile.TemporaryDirectory() as temporary:
            result = capture_failure.capture(
                self.failing_command(),
                case_id="CAPTURE_STABLE",
                output_root=Path(temporary) / "output",
                repository_root=self.repository_root(temporary),
                source_identity_provider=lambda: next(snapshots),
            )

        self.assertEqual(result.status, failure_packet.CAPTURE_COMMAND_EXIT)


class RerunFailureTests(unittest.TestCase):
    @staticmethod
    def repository_root(temporary: str) -> Path:
        repository_root = Path(temporary) / "repo"
        repository_root.mkdir()
        return repository_root

    @staticmethod
    def identity(
        *,
        commit: str = "a" * 40,
        tree: str = "b" * 40,
        fingerprint: str = "c" * 64,
        clean: bool = True,
    ) -> failure_packet.SourceIdentity:
        return failure_packet.SourceIdentity(
            commit=commit,
            tree=tree,
            fingerprint=fingerprint,
            clean=clean,
        )

    @staticmethod
    def command(exit_status: int = 1) -> list[str]:
        return [
            sys.executable,
            "-c",
            f"import sys; sys.exit({exit_status})",
        ]

    @staticmethod
    def marker_line(
        *,
        surface: str = "events",
        semantic_path: str = "transition.events[3]",
        mismatch_kind: str = "value_changed",
    ) -> str:
        return (
            "MANAFOLD_FAILURE_SIGNATURE v1 "
            f"surface={surface} path={semantic_path} mismatch_kind={mismatch_kind}"
        )

    def make_packet(
        self,
        temporary: str,
        *,
        outcome: str = failure_packet.COMMAND_EXIT,
        exit_status: int | None = 1,
        argv: list[str] | None = None,
        cwd: str = ".",
        marker: dict[str, str] | None = None,
        tool_version: str | None = None,
    ) -> tuple[Path, Path, failure_packet.SourceIdentity]:
        repository_root = self.repository_root(temporary)
        identity = self.identity()
        argv = list(argv or self.command())
        manifest = base_manifest()
        manifest["packet_id"] = "rerun-case"
        manifest["source"] = {
            "commit": identity.commit,
            "tree": identity.tree,
            "fingerprint": identity.fingerprint,
            "clean": identity.clean,
        }
        manifest["command"] = {"argv": argv, "cwd": cwd}
        manifest["execution"] = {
            "outcome": outcome,
            "exit_status": exit_status,
            "timeout_seconds": failure_packet.MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS,
        }
        manifest["failure"] = {"classification": outcome}
        manifest["failure_signature"] = {
            "origin_kind": "command",
            "case_id": "CASE_1",
            "failure_classification": outcome,
            "surface": marker["surface"] if marker else None,
            "semantic_path": marker["semantic_path"] if marker else None,
            "mismatch_kind": marker["mismatch_kind"] if marker else None,
        }
        manifest["tools"] = failure_packet.tool_identity()
        if tool_version is not None:
            manifest["tools"]["capture_python"]["version"] = tool_version
        log = b"failure\n"
        if marker is not None:
            log = (self.marker_line(**marker) + "\n").encode("utf-8")
        packet = failure_packet.write_packet(
            Path(temporary) / "output",
            "rerun-case",
            manifest,
            log,
        )
        return packet, repository_root, identity

    def rerun_with_outcome(
        self,
        rerun_failure: Any,
        packet: Path,
        repository_root: Path,
        identity: failure_packet.SourceIdentity,
        outcome: failure_packet.CommandOutcome,
    ) -> tuple[Any, mock.MagicMock]:
        run_bounded = mock.patch.object(
            rerun_failure,
            "run_bounded",
            return_value=outcome,
        )
        started = run_bounded.start()
        try:
            result = rerun_failure.rerun(
                packet,
                repository_root=repository_root,
                source_identity_provider=lambda: identity,
            )
        finally:
            run_bounded.stop()
        return result, started

    def test_matching_command_exit_is_reproduced(self) -> None:
        import rerun_failure

        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(temporary)
            result, run_bounded = self.rerun_with_outcome(
                rerun_failure,
                packet,
                repository_root,
                identity,
                failure_packet.CommandOutcome(returncode=1),
            )

        self.assertEqual(result.status, failure_packet.RERUN_REPRODUCED)
        self.assertEqual(result.exit_code, 0)
        run_bounded.assert_called_once()
        self.assertEqual(run_bounded.call_args.args[0], self.command())
        self.assertEqual(run_bounded.call_args.kwargs["cwd"], repository_root.resolve())
        self.assertFalse(run_bounded.call_args.kwargs["shell"])
        self.assertLessEqual(
            run_bounded.call_args.kwargs["timeout"],
            failure_packet.MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS,
        )

    def test_different_exit_status_is_not_reproduced(self) -> None:
        import rerun_failure

        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(temporary)
            result, _run_bounded = self.rerun_with_outcome(
                rerun_failure,
                packet,
                repository_root,
                identity,
                failure_packet.CommandOutcome(returncode=2),
            )

        self.assertEqual(result.status, failure_packet.RERUN_NOT_REPRODUCED)
        self.assertEqual(result.exit_code, 1)

    def test_passing_rerun_is_not_reproduced(self) -> None:
        import rerun_failure

        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(temporary)
            result, _run_bounded = self.rerun_with_outcome(
                rerun_failure,
                packet,
                repository_root,
                identity,
                failure_packet.CommandOutcome(returncode=0),
            )

        self.assertEqual(result.status, failure_packet.RERUN_NOT_REPRODUCED)
        self.assertEqual(result.exit_code, 1)

    def test_matching_timeout_is_reproduced(self) -> None:
        import rerun_failure

        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(
                temporary,
                outcome=failure_packet.COMMAND_TIMEOUT,
                exit_status=None,
            )
            result, _run_bounded = self.rerun_with_outcome(
                rerun_failure,
                packet,
                repository_root,
                identity,
                failure_packet.CommandOutcome(returncode=None, timed_out=True),
            )

        self.assertEqual(result.status, failure_packet.RERUN_REPRODUCED)
        self.assertEqual(result.exit_code, 0)

    def test_timeout_followed_by_ordinary_exit_is_not_reproduced(self) -> None:
        import rerun_failure

        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(
                temporary,
                outcome=failure_packet.COMMAND_TIMEOUT,
                exit_status=None,
            )
            result, _run_bounded = self.rerun_with_outcome(
                rerun_failure,
                packet,
                repository_root,
                identity,
                failure_packet.CommandOutcome(returncode=1),
            )

        self.assertEqual(result.status, failure_packet.RERUN_NOT_REPRODUCED)
        self.assertEqual(result.exit_code, 1)

    def test_matching_structured_signature_is_reproduced(self) -> None:
        import rerun_failure

        marker = {
            "surface": "events",
            "semantic_path": "transition.events[3]",
            "mismatch_kind": "value_changed",
        }
        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(
                temporary,
                marker=marker,
            )
            result, _run_bounded = self.rerun_with_outcome(
                rerun_failure,
                packet,
                repository_root,
                identity,
                failure_packet.CommandOutcome(
                    returncode=1,
                    stdout=(self.marker_line(**marker) + "\n").encode("utf-8"),
                ),
            )

        self.assertEqual(result.status, failure_packet.RERUN_REPRODUCED)

    def test_different_structured_signature_is_not_reproduced(self) -> None:
        import rerun_failure

        expected = {
            "surface": "events",
            "semantic_path": "transition.events[3]",
            "mismatch_kind": "value_changed",
        }
        actual = {
            "surface": "delta",
            "semantic_path": "transition.delta.audit[1]",
            "mismatch_kind": "value_changed",
        }
        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(
                temporary,
                marker=expected,
            )
            result, _run_bounded = self.rerun_with_outcome(
                rerun_failure,
                packet,
                repository_root,
                identity,
                failure_packet.CommandOutcome(
                    returncode=1,
                    stdout=(self.marker_line(**actual) + "\n").encode("utf-8"),
                ),
            )

        self.assertEqual(result.status, failure_packet.RERUN_NOT_REPRODUCED)
        self.assertEqual(result.exit_code, 1)

    def test_structured_signature_without_marker_is_blocked(self) -> None:
        import rerun_failure

        marker = {
            "surface": "events",
            "semantic_path": "transition.events[3]",
            "mismatch_kind": "value_changed",
        }
        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(
                temporary,
                marker=marker,
            )
            result, _run_bounded = self.rerun_with_outcome(
                rerun_failure,
                packet,
                repository_root,
                identity,
                failure_packet.CommandOutcome(returncode=1),
            )

        self.assertEqual(result.status, failure_packet.RERUN_BLOCKED)
        self.assertEqual(result.exit_code, 2)

    def test_malformed_rerun_marker_is_blocked(self) -> None:
        import rerun_failure

        marker = {
            "surface": "events",
            "semantic_path": "transition.events[3]",
            "mismatch_kind": "value_changed",
        }
        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(
                temporary,
                marker=marker,
            )
            result, _run_bounded = self.rerun_with_outcome(
                rerun_failure,
                packet,
                repository_root,
                identity,
                failure_packet.CommandOutcome(
                    returncode=1,
                    stdout=b"MANAFOLD_FAILURE_SIGNATURE v1 malformed\n",
                ),
            )

        self.assertEqual(result.status, failure_packet.RERUN_BLOCKED)
        self.assertEqual(result.exit_code, 2)

    def test_generic_signature_with_marker_is_not_reproduced(self) -> None:
        import rerun_failure

        marker = {
            "surface": "events",
            "semantic_path": "transition.events[3]",
            "mismatch_kind": "value_changed",
        }
        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(temporary)
            result, _run_bounded = self.rerun_with_outcome(
                rerun_failure,
                packet,
                repository_root,
                identity,
                failure_packet.CommandOutcome(
                    returncode=1,
                    stdout=(self.marker_line(**marker) + "\n").encode("utf-8"),
                ),
            )

        self.assertEqual(result.status, failure_packet.RERUN_NOT_REPRODUCED)

    def test_source_identity_mismatch_blocks_before_command(self) -> None:
        import rerun_failure

        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(temporary)
            changed = self.identity(commit="d" * 40)
            with mock.patch.object(rerun_failure, "run_bounded") as run_bounded:
                result = rerun_failure.rerun(
                    packet,
                    repository_root=repository_root,
                    source_identity_provider=lambda: changed,
                )

        self.assertEqual(result.status, failure_packet.RERUN_BLOCKED)
        self.assertEqual(result.exit_code, 2)
        run_bounded.assert_not_called()
        self.assertNotEqual(identity.commit, changed.commit)

    def test_each_source_identity_mismatch_blocks_before_command(self) -> None:
        import rerun_failure

        changed_identities = {
            "commit": self.identity(commit="d" * 40),
            "tree": self.identity(tree="d" * 40),
            "fingerprint": self.identity(fingerprint="d" * 64),
            "clean": self.identity(clean=False),
        }
        for field, changed in changed_identities.items():
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temporary:
                packet, repository_root, _identity = self.make_packet(temporary)
                with mock.patch.object(rerun_failure, "run_bounded") as run_bounded:
                    result = rerun_failure.rerun(
                        packet,
                        repository_root=repository_root,
                        source_identity_provider=lambda changed=changed: changed,
                    )

            self.assertEqual(result.status, failure_packet.RERUN_BLOCKED)
            self.assertEqual(result.exit_code, 2)
            run_bounded.assert_not_called()

    def test_tool_version_mismatch_blocks_before_command(self) -> None:
        import rerun_failure

        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(
                temporary,
                tool_version="0.0.0-test-mismatch",
            )
            with mock.patch.object(rerun_failure, "run_bounded") as run_bounded:
                result = rerun_failure.rerun(
                    packet,
                    repository_root=repository_root,
                    source_identity_provider=lambda: identity,
                )

        self.assertEqual(result.status, failure_packet.RERUN_BLOCKED)
        self.assertEqual(result.exit_code, 2)
        run_bounded.assert_not_called()

    def test_missing_cwd_blocks_before_command(self) -> None:
        import rerun_failure

        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(
                temporary,
                cwd="missing-directory",
            )
            with mock.patch.object(rerun_failure, "run_bounded") as run_bounded:
                result = rerun_failure.rerun(
                    packet,
                    repository_root=repository_root,
                    source_identity_provider=lambda: identity,
                )

        self.assertEqual(result.status, failure_packet.RERUN_BLOCKED)
        self.assertEqual(result.exit_code, 2)
        run_bounded.assert_not_called()

    def test_missing_executable_blocks_before_command(self) -> None:
        import rerun_failure

        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(
                temporary,
                argv=[str(Path(temporary) / "repo" / "missing-command.exe")],
            )
            with mock.patch.object(rerun_failure, "run_bounded") as run_bounded:
                result = rerun_failure.rerun(
                    packet,
                    repository_root=repository_root,
                    source_identity_provider=lambda: identity,
                )

        self.assertEqual(result.status, failure_packet.RERUN_BLOCKED)
        self.assertEqual(result.exit_code, 2)
        run_bounded.assert_not_called()

    def test_source_mutation_blocks_even_matching_failure(self) -> None:
        import rerun_failure

        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(temporary)
            snapshots = iter([identity, self.identity(fingerprint="d" * 64)])
            with mock.patch.object(
                rerun_failure,
                "run_bounded",
                return_value=failure_packet.CommandOutcome(returncode=1),
            ) as run_bounded:
                result = rerun_failure.rerun(
                    packet,
                    repository_root=repository_root,
                    source_identity_provider=lambda: next(snapshots),
                )

        self.assertEqual(result.status, failure_packet.RERUN_BLOCKED)
        self.assertEqual(result.exit_code, 2)
        run_bounded.assert_called_once()

    def test_tampered_nested_packet_blocks_before_command(self) -> None:
        import rerun_failure

        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(temporary)
            manifest_path = packet / "manifest.json"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["command"]["environment"] = {
                "TOKEN": "SECRET_ENV_SENTINEL",
            }
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            with mock.patch.object(rerun_failure, "run_bounded") as run_bounded:
                result = rerun_failure.rerun(
                    packet,
                    repository_root=repository_root,
                    source_identity_provider=lambda: identity,
                )

        self.assertEqual(result.status, failure_packet.RERUN_BLOCKED)
        self.assertEqual(result.exit_code, 2)
        run_bounded.assert_not_called()

    def test_tampered_log_checksum_blocks_before_command(self) -> None:
        import rerun_failure

        with tempfile.TemporaryDirectory() as temporary:
            packet, repository_root, identity = self.make_packet(temporary)
            (packet / "command.log").write_bytes(b"tampered\n")
            with mock.patch.object(rerun_failure, "run_bounded") as run_bounded:
                result = rerun_failure.rerun(
                    packet,
                    repository_root=repository_root,
                    source_identity_provider=lambda: identity,
                )

        self.assertEqual(result.status, failure_packet.RERUN_BLOCKED)
        self.assertEqual(result.exit_code, 2)
        run_bounded.assert_not_called()


class T0FailureContextTests(unittest.TestCase):
    """T0 failure-context binding: the closed context line carries case,
    step, diagnostic, authority, kernel, and expected/actual digest
    identities. Capture binds the declared case id; rerun requires exact
    context equality."""

    CASE = "synthetic-entry-digest-mismatch"
    CONTEXT = (
        "T0_FAILURE_CONTEXT v1 case=synthetic-entry-digest-mismatch "
        "step=step-1-entry-choose-one index=0 surface=state_digest "
        "path=transition.state_digest kind=value_changed "
        "authority=mtgml_conformance::assert_exact_transition "
        "kernel=synthetic-m2 semantic=0.2.2 "
        "expected_state_digest=03e13400f71135656196ea83b00bad1821df341ea8b9a03634e45fc0ae83ed0a "
        "actual_state_digest=4bf8babd2e5661bd64e84e4830dae09c633d0db348e4df7e75915a15c6c1d3e7"
    )
    SIGNATURE = (
        "MANAFOLD_FAILURE_SIGNATURE v1 surface=state_digest "
        "path=transition.state_digest mismatch_kind=value_changed"
    )

    @staticmethod
    def repository_root(temporary: str) -> Path:
        repository_root = Path(temporary) / "repo"
        repository_root.mkdir()
        return repository_root

    @staticmethod
    def identity_provider() -> failure_packet.SourceIdentity:
        return failure_packet.SourceIdentity(
            commit="a" * 40,
            tree="b" * 40,
            fingerprint="c" * 64,
            clean=True,
        )

    @classmethod
    def witness_command(
        cls, *, context: str | None = None, exit_status: int = 1
    ) -> list[str]:
        lines = context if context is not None else cls.CONTEXT
        return [
            sys.executable,
            "-c",
            (
                f"print({lines!r}); "
                f"print({cls.SIGNATURE!r}); "
                f"raise SystemExit({exit_status})"
            ),
        ]

    def test_t0_context_parses_valid_line(self) -> None:
        context = failure_packet.parse_t0_failure_context((self.CONTEXT + "\n").encode())
        self.assertEqual(context["case"], self.CASE)
        self.assertEqual(context["step"], "step-1-entry-choose-one")
        self.assertEqual(context["index"], "0")
        self.assertEqual(context["surface"], "state_digest")
        self.assertEqual(
            context["expected_state_digest"],
            "03e13400f71135656196ea83b00bad1821df341ea8b9a03634e45fc0ae83ed0a",
        )
        self.assertEqual(
            context["actual_state_digest"],
            "4bf8babd2e5661bd64e84e4830dae09c633d0db348e4df7e75915a15c6c1d3e7",
        )
        self.assertEqual(len(context), 11)

    def test_t0_context_absent_returns_none(self) -> None:
        self.assertIsNone(failure_packet.parse_t0_failure_context(b"no markers here\n"))

    def test_t0_context_rejects_multiple_lines(self) -> None:
        with self.assertRaises(failure_packet.FailurePacketError):
            failure_packet.parse_t0_failure_context(
                (self.CONTEXT + "\n" + self.CONTEXT + "\n").encode()
            )

    def test_t0_context_rejects_malformed_line(self) -> None:
        with self.assertRaises(failure_packet.FailurePacketError):
            failure_packet.parse_t0_failure_context(b"T0_FAILURE_CONTEXT v1 nope\n")

    def test_t0_context_rejects_bad_digest(self) -> None:
        bad = self.CONTEXT.replace("expected_state_digest=03e1", "expected_state_digest=ZZZZ")
        with self.assertRaises(failure_packet.FailurePacketError):
            failure_packet.parse_t0_failure_context((bad + "\n").encode())

    def test_t0_context_rejects_unknown_surface(self) -> None:
        bad = self.CONTEXT.replace("surface=state_digest", "surface=made_up")
        with self.assertRaises(failure_packet.FailurePacketError):
            failure_packet.parse_t0_failure_context((bad + "\n").encode())

    def test_matching_context_case_id_creates_packet(self) -> None:
        import capture_failure

        with tempfile.TemporaryDirectory() as temporary:
            result = capture_failure.capture(
                self.witness_command(),
                case_id=self.CASE,
                output_root=Path(temporary) / "output",
                repository_root=self.repository_root(temporary),
                source_identity_provider=self.identity_provider,
            )
            manifest = failure_packet.load_packet(result.packet)

        self.assertEqual(result.status, failure_packet.CAPTURE_COMMAND_EXIT)
        self.assertIsNotNone(result.packet)
        self.assertEqual(
            manifest["failure_signature"]["semantic_path"],
            "transition.state_digest",
        )

    def test_mismatching_context_case_id_is_blocked_without_packet(self) -> None:
        import capture_failure

        with tempfile.TemporaryDirectory() as temporary:
            result = capture_failure.capture(
                self.witness_command(),
                case_id="DECLARED_CASE",
                output_root=Path(temporary) / "output",
                repository_root=self.repository_root(temporary),
                source_identity_provider=self.identity_provider,
            )

        self.assertEqual(result.status, failure_packet.CAPTURE_BLOCKED)
        self.assertEqual(result.exit_code, 2)
        self.assertIsNone(result.packet)

    def test_t0_context_without_signature_is_blocked(self) -> None:
        import capture_failure

        command = [
            sys.executable,
            "-c",
            f"print({self.CONTEXT!r}); raise SystemExit(1)",
        ]
        with tempfile.TemporaryDirectory() as temporary:
            result = capture_failure.capture(
                command,
                case_id=self.CASE,
                output_root=Path(temporary) / "output",
                repository_root=self.repository_root(temporary),
                source_identity_provider=self.identity_provider,
            )

        self.assertEqual(result.status, failure_packet.CAPTURE_BLOCKED)
        self.assertEqual(result.exit_code, 2)
        self.assertIsNone(result.packet)

    def test_t0_context_disagreeing_with_signature_is_blocked(self) -> None:
        import capture_failure

        drifted_signature = self.SIGNATURE.replace(
            "surface=state_digest", "surface=events"
        )
        command = [
            sys.executable,
            "-c",
            (
                f"print({self.CONTEXT!r}); "
                f"print({drifted_signature!r}); "
                "raise SystemExit(1)"
            ),
        ]
        with tempfile.TemporaryDirectory() as temporary:
            result = capture_failure.capture(
                command,
                case_id=self.CASE,
                output_root=Path(temporary) / "output",
                repository_root=self.repository_root(temporary),
                source_identity_provider=self.identity_provider,
            )

        self.assertEqual(result.status, failure_packet.CAPTURE_BLOCKED)
        self.assertEqual(result.exit_code, 2)
        self.assertIsNone(result.packet)

    def test_rerun_reproduces_identical_t0_context(self) -> None:
        import capture_failure
        import rerun_failure

        with tempfile.TemporaryDirectory() as temporary:
            repository_root = self.repository_root(temporary)
            identity = self.identity_provider()
            captured = capture_failure.capture(
                self.witness_command(),
                case_id=self.CASE,
                output_root=Path(temporary) / "output",
                repository_root=repository_root,
                source_identity_provider=lambda: identity,
            )
            self.assertEqual(captured.status, failure_packet.CAPTURE_COMMAND_EXIT)
            outcome = failure_packet.CommandOutcome(
                returncode=1,
                stdout=(self.CONTEXT + "\n" + self.SIGNATURE + "\n").encode(),
            )
            with mock.patch.object(
                rerun_failure, "run_bounded", return_value=outcome
            ):
                result = rerun_failure.rerun(
                    captured.packet,
                    repository_root=repository_root,
                    source_identity_provider=lambda: identity,
                )

        self.assertEqual(result.status, failure_packet.RERUN_REPRODUCED)
        self.assertEqual(result.exit_code, 0)

    def test_rerun_with_differing_actual_digest_is_not_reproduced(self) -> None:
        import capture_failure
        import rerun_failure

        with tempfile.TemporaryDirectory() as temporary:
            repository_root = self.repository_root(temporary)
            identity = self.identity_provider()
            captured = capture_failure.capture(
                self.witness_command(),
                case_id=self.CASE,
                output_root=Path(temporary) / "output",
                repository_root=repository_root,
                source_identity_provider=lambda: identity,
            )
            self.assertEqual(captured.status, failure_packet.CAPTURE_COMMAND_EXIT)
            drifted = self.CONTEXT.replace(
                "actual_state_digest=4bf8babd2e5661bd64e84e4830dae09c633d0db348e4df7e75915a15c6c1d3e7",
                "actual_state_digest=" + "0" * 64,
            )
            outcome = failure_packet.CommandOutcome(
                returncode=1,
                stdout=(drifted + "\n" + self.SIGNATURE + "\n").encode(),
            )
            with mock.patch.object(
                rerun_failure, "run_bounded", return_value=outcome
            ):
                result = rerun_failure.rerun(
                    captured.packet,
                    repository_root=repository_root,
                    source_identity_provider=lambda: identity,
                )

        self.assertEqual(result.status, failure_packet.RERUN_NOT_REPRODUCED)
        self.assertEqual(result.exit_code, 1)

    def test_rerun_missing_t0_context_is_blocked(self) -> None:
        import capture_failure
        import rerun_failure

        with tempfile.TemporaryDirectory() as temporary:
            repository_root = self.repository_root(temporary)
            identity = self.identity_provider()
            captured = capture_failure.capture(
                self.witness_command(),
                case_id=self.CASE,
                output_root=Path(temporary) / "output",
                repository_root=repository_root,
                source_identity_provider=lambda: identity,
            )
            self.assertEqual(captured.status, failure_packet.CAPTURE_COMMAND_EXIT)
            outcome = failure_packet.CommandOutcome(
                returncode=1,
                stdout=(self.SIGNATURE + "\n").encode(),
            )
            with mock.patch.object(
                rerun_failure, "run_bounded", return_value=outcome
            ):
                result = rerun_failure.rerun(
                    captured.packet,
                    repository_root=repository_root,
                    source_identity_provider=lambda: identity,
                )

        self.assertEqual(result.status, failure_packet.RERUN_BLOCKED)
        self.assertEqual(result.exit_code, 2)


if __name__ == "__main__":
    unittest.main()
