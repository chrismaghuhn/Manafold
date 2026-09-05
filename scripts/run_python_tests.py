#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sys
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
SOURCE_ROOT = ROOT / "python" / "src"
TESTS_ROOT = ROOT / "python" / "tests"

DEFAULT_PROFILE = "full"

# Keep this list intentionally explicit. New tests enter the full profile by
# default and only enter Smoke after an intentional maintainer decision.
SMOKE_TESTS = (
    "test_authority_contract",
    "test_context_application_v2_contract",
    "test_constructive_producers",
    "test_m2_b_staging_fixtures",
    "test_persistence_codec",
    "test_python_test_profiles",
    "test_review_admission_foundation",
    "test_schema_parity",
    "test_wire_contracts",
)


def configure_source_import_path() -> None:
    """Make the repository's src-layout packages importable before discovery."""

    sys.path.insert(0, str(SOURCE_ROOT))
    sys.path.insert(1, str(TESTS_ROOT))


def build_suite(profile: str) -> unittest.TestSuite:
    """Build either the complete suite or the explicit development smoke suite."""

    if profile == "full":
        return unittest.defaultTestLoader.discover(str(TESTS_ROOT))
    if profile == "smoke":
        return unittest.TestSuite(
            unittest.defaultTestLoader.loadTestsFromName(name) for name in SMOKE_TESTS
        )
    raise ValueError(f"unknown Python test profile: {profile}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", choices=("smoke", "full"), default=DEFAULT_PROFILE)
    args = parser.parse_args()

    configure_source_import_path()
    suite = build_suite(args.profile)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
