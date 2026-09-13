from __future__ import annotations

import contextlib
import io
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import ClassVar
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import bootstrap
import doctor


class BootstrapTests(unittest.TestCase):
    REQUIRED_PYTHON = "3.13.15"

    @staticmethod
    def _create_fake_venv(path: Path) -> None:
        python = bootstrap.venv_python_path(path)
        python.parent.mkdir(parents=True, exist_ok=True)
        python.touch()

    def _run_bootstrap(self, venv_path: Path) -> int:
        with mock.patch.object(sys, "argv", ["bootstrap.py", "--venv", str(venv_path)]):
            return bootstrap.main()

    def test_wrong_python_fails_before_venv_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            venv_path = Path(directory) / ".venv"
            with (
                mock.patch.object(bootstrap, "current_python_version", return_value="3.14.5"),
                mock.patch.object(bootstrap.venv, "EnvBuilder") as builder,
                mock.patch.object(bootstrap, "run_command") as run_command,
            ):
                result = self._run_bootstrap(venv_path)

        self.assertNotEqual(result, 0)
        self.assertFalse(venv_path.exists())
        builder.assert_not_called()
        run_command.assert_not_called()

    def test_exact_python_bootstrap_is_idempotent_and_does_not_touch_lock_commands(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            venv_path = Path(directory) / ".venv"
            builder = mock.Mock()
            builder.create.side_effect = self._create_fake_venv
            protected_before = {
                relative: (ROOT / relative).read_bytes() for relative in bootstrap.PROTECTED_FILES
            }
            with (
                mock.patch.object(
                    bootstrap, "current_python_version", return_value=self.REQUIRED_PYTHON
                ),
                mock.patch.object(bootstrap.venv, "EnvBuilder", return_value=builder),
                mock.patch.object(
                    bootstrap,
                    "probe_python_version",
                    return_value=self.REQUIRED_PYTHON,
                ),
                mock.patch.object(bootstrap, "run_command", return_value=0) as run_command,
            ):
                first = self._run_bootstrap(venv_path)
                second = self._run_bootstrap(venv_path)

        self.assertEqual(first, 0)
        self.assertEqual(second, 0)
        builder.create.assert_called_once_with(venv_path)
        self.assertEqual(
            protected_before,
            {relative: (ROOT / relative).read_bytes() for relative in bootstrap.PROTECTED_FILES},
        )
        commands = [call.args[0] for call in run_command.call_args_list]
        self.assertTrue(commands)
        self.assertTrue(all("Cargo.lock" not in command for command in commands))
        self.assertTrue(all("rustup" not in command for command in commands))

    def test_install_failure_is_nonzero_and_stops_without_retry(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            venv_path = Path(directory) / ".venv"
            builder = mock.Mock()
            builder.create.side_effect = self._create_fake_venv
            with (
                mock.patch.object(
                    bootstrap, "current_python_version", return_value=self.REQUIRED_PYTHON
                ),
                mock.patch.object(bootstrap.venv, "EnvBuilder", return_value=builder),
                mock.patch.object(
                    bootstrap,
                    "probe_python_version",
                    return_value=self.REQUIRED_PYTHON,
                ),
                mock.patch.object(bootstrap, "run_command", side_effect=[0, 17]) as run_command,
            ):
                result = self._run_bootstrap(venv_path)

        self.assertEqual(result, 17)
        self.assertEqual(run_command.call_count, 2)

    def test_subprocess_timeout_fails_closed_with_rerun_diagnostic(self) -> None:
        command = ["python", "-m", "pip", "install"]
        stdout = io.StringIO()
        stderr = io.StringIO()
        with (
            mock.patch.object(
                bootstrap.subprocess,
                "run",
                side_effect=subprocess.TimeoutExpired(command, 600),
            ),
            contextlib.redirect_stdout(stdout),
            contextlib.redirect_stderr(stderr),
        ):
            result = bootstrap.run_command(command)

        self.assertEqual(result, 124)
        self.assertIn("TIMEOUT", stderr.getvalue())
        self.assertIn("pip install", stderr.getvalue())
        self.assertIn("limit=600s", stderr.getvalue())
        self.assertIn("RERUN:", stderr.getvalue())

    def test_bootstrap_subprocess_budget_is_at_most_ten_minutes(self) -> None:
        self.assertLessEqual(bootstrap.MAX_BOOTSTRAP_SUBPROCESS_RUNTIME_SECONDS, 600)


class DoctorTests(unittest.TestCase):
    EXECUTABLE = "C:/project/.venv/Scripts/python.exe"
    TOOL_VERSIONS: ClassVar[dict[str, str]] = {
        "ruff": "0.12.9",
        "mypy": "1.17.1",
        "pytest": "8.4.1",
        "jsonschema": "4.26.0",
        "referencing": "0.36.2",
        "pyyaml": "6.0.3",
    }

    @contextlib.contextmanager
    def healthy_environment(self, *, python_version: str = "3.13.15"):
        native_calls: list[str] = []

        def which(name: str) -> str:
            native_calls.append(name)
            return f"C:/tools/{name}.exe"

        def distribution_version(name: str) -> str:
            return self.TOOL_VERSIONS[name.lower()]

        def probe(command: list[str]) -> str:
            if command[:3] == ["rustup", "show", "active-toolchain"]:
                return "1.85.1-x86_64-pc-windows-msvc (overridden)"
            if command and command[0] == "rustc":
                return "rustc 1.85.1 (test)"
            if command and command[0] == "cargo":
                return "cargo 1.85.1 (test)"
            if command and command[0] == "rustfmt":
                return "rustfmt 1.8.0-stable (test)"
            if command and command[0] == "clippy-driver":
                return "clippy 0.1.85 (test)"
            if len(command) >= 3 and command[0] == self.EXECUTABLE:
                return f"{command[2]} {self.TOOL_VERSIONS[command[2]]}"
            return "probe ok"

        with (
            mock.patch.object(doctor.sys, "platform", "win32"),
            mock.patch.object(doctor.sys, "executable", self.EXECUTABLE),
            mock.patch.object(doctor.platform, "platform", return_value="Windows-test"),
            mock.patch.object(doctor.platform, "python_version", return_value=python_version),
            mock.patch.object(doctor, "project_python_path", return_value=Path(self.EXECUTABLE)),
            mock.patch.object(doctor.shutil, "which", side_effect=which),
            mock.patch.object(doctor.importlib.util, "find_spec", return_value=object()),
            mock.patch.object(doctor.metadata, "version", side_effect=distribution_version),
            mock.patch.object(doctor, "probe", side_effect=probe),
        ):
            yield native_calls

    def _run_doctor(self, *arguments: str) -> tuple[int, str]:
        stdout = io.StringIO()
        with (
            mock.patch.object(sys, "argv", ["doctor.py", *arguments]),
            contextlib.redirect_stdout(stdout),
        ):
            result = doctor.main()
        return result, stdout.getvalue()

    def test_strict_accepts_intended_project_environment(self) -> None:
        with self.healthy_environment() as native_calls:
            result, output = self._run_doctor("--strict")

        self.assertEqual(result, 0)
        self.assertIn("project .venv: YES", output)
        self.assertIn("selected Python tools", output)
        self.assertNotIn("ruff", native_calls)
        self.assertNotIn("mypy", native_calls)
        self.assertNotIn("pytest", native_calls)

    def test_strict_rejects_wrong_python(self) -> None:
        with self.healthy_environment(python_version="3.14.5"):
            result, output = self._run_doctor("--strict")

        self.assertNotEqual(result, 0)
        self.assertIn("python", output.lower())
        self.assertIn("3.13.15", output)

    def test_strict_rejects_wrong_rust(self) -> None:
        with self.healthy_environment(), mock.patch.object(doctor, "probe") as probe:
            probe.side_effect = lambda command: (
                "rustc 1.84.0 (wrong)" if command and command[0] == "rustc" else "probe ok"
            )
            result, output = self._run_doctor("--strict")

        self.assertNotEqual(result, 0)
        self.assertIn("rust", output.lower())

    def test_strict_rejects_missing_project_tooling(self) -> None:
        with (
            self.healthy_environment(),
            mock.patch.object(
                doctor.metadata,
                "version",
                side_effect=lambda name: (_ for _ in ()).throw(
                    doctor.metadata.PackageNotFoundError(name)
                )
                if name.lower() == "ruff"
                else self.TOOL_VERSIONS[name.lower()],
            ),
            mock.patch.object(
                doctor,
                "probe",
                side_effect=lambda command: None
                if len(command) >= 3 and command[2] == "ruff"
                else "probe ok",
            ),
        ):
            result, output = self._run_doctor("--strict")

        self.assertNotEqual(result, 0)
        self.assertIn("ruff", output.lower())

    def test_probe_timeout_is_bounded_and_diagnostic(self) -> None:
        command = ["rustc", "--version"]
        stderr = io.StringIO()
        with (
            mock.patch.object(
                doctor.subprocess,
                "run",
                side_effect=subprocess.TimeoutExpired(command, doctor.DOCTOR_PROBE_TIMEOUT_SECONDS),
            ),
            contextlib.redirect_stderr(stderr),
        ):
            result = doctor.probe(command)

        self.assertIsNone(result)
        self.assertIn("TIMEOUT", stderr.getvalue())
        self.assertIn("rustc --version", stderr.getvalue())
        self.assertIn("RERUN:", stderr.getvalue())

    def test_strict_reports_missing_native_tool(self) -> None:
        with (
            self.healthy_environment(),
            mock.patch.object(
                doctor.shutil,
                "which",
                side_effect=lambda name: None if name == "cargo" else f"C:/tools/{name}.exe",
            ),
        ):
            result, output = self._run_doctor("--strict")

        self.assertNotEqual(result, 0)
        self.assertIn("cargo", output.lower())

    def test_doctor_is_non_mutating(self) -> None:
        paths = [ROOT / ".python-version", ROOT / "rust-toolchain.toml", ROOT / "Cargo.lock"]
        before = {path: path.read_bytes() for path in paths}
        with self.healthy_environment():
            result, _ = self._run_doctor("--strict")

        self.assertEqual(result, 0)
        self.assertEqual(before, {path: path.read_bytes() for path in paths})

    def test_doctor_probe_budget_is_small(self) -> None:
        self.assertLessEqual(doctor.DOCTOR_PROBE_TIMEOUT_SECONDS, 60)


if __name__ == "__main__":
    unittest.main()
