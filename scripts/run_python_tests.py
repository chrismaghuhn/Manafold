#!/usr/bin/env python3
from __future__ import annotations
import sys
import unittest
from pathlib import Path
sys.dont_write_bytecode = True
root = Path(__file__).resolve().parents[1]
suite = unittest.defaultTestLoader.discover(str(root / "python" / "tests"))
result = unittest.TextTestRunner(verbosity=2).run(suite)
raise SystemExit(0 if result.wasSuccessful() else 1)
