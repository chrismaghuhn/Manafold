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
        manifest["failure"]["expected_summary"] = "INTERNAL_OBJECT_ID_SENTINEL"
        manifest["failure"]["actual_summary"] = "PRIVATE_HAND_SENTINEL"
        manifest["tools"]["private_path"] = "ROOT_SEED_SECRET_SENTINEL"
        manifest["failure_signature"]["semantic_path"] = (
            "zones.locations[object:INTERNAL_OBJECT_ID_SENTINEL]"
        )
        manifest["failure_signature"]["private_detail"] = "ROOT_SEED_SECRET_SENTINEL"

        summary = failure_packet.safe_summary(manifest)
        rendered = json.dumps(summary, sort_keys=True)

        self.assertNotIn("ROOT_SEED_SECRET_SENTINEL", rendered)
        self.assertNotIn("INTERNAL_OBJECT_ID_SENTINEL", rendered)
        self.assertNotIn("PRIVATE_HAND_SENTINEL", rendered)
        self.assertEqual(summary["failure"]["classification"], "COMMAND_EXIT")

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


if __name__ == "__main__":
    unittest.main()


if __name__ == "__main__":
    unittest.main()
