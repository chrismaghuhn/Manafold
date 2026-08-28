#!/usr/bin/env python3
from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
SOURCE_ROOT = ROOT / "python" / "src"


def configure_source_import_path() -> None:
    """Make the repository's src-layout packages importable before discovery."""

    sys.path.insert(0, str(SOURCE_ROOT))


def main() -> int:
    configure_source_import_path()
    suite = unittest.defaultTestLoader.discover(str(ROOT / "python" / "tests"))
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
