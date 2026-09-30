from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.errors import WireError
from mtgml.wire import decode_canonical, encode_canonical

DELETED_CONTRACTS: tuple[str, ...] = tuple(
    f"{name}.v{version}"
    for name in ("replay-manifest", "authoritative-replay")
    for version in range(1, 8)
)


class DeletedContractTests(unittest.TestCase):
    def test_deleted_contracts_are_unknown(self) -> None:
        for contract in DELETED_CONTRACTS:
            with self.subTest(contract=contract):
                with self.assertRaises(WireError) as caught:
                    decode_canonical(contract, b"{}")
                self.assertEqual(caught.exception.code, "fixture.unknown_contract")


class SharedFixtureTests(unittest.TestCase):
    def test_every_golden_fixture_roundtrips_to_identical_bytes(self) -> None:
        directory = ROOT / "wire" / "golden"
        manifest = json.loads((directory / "manifest.json").read_text(encoding="utf-8"))
        self.assertGreaterEqual(len(manifest["fixtures"]), 10)
        for case in manifest["fixtures"]:
            with self.subTest(case=case["path"]):
                payload = (directory / case["path"]).read_bytes()
                decoded = decode_canonical(case["contract"], payload)
                self.assertEqual(encode_canonical(decoded), payload)

    def test_every_negative_fixture_is_rejected_with_expected_code(self) -> None:
        directory = ROOT / "wire" / "negative"
        manifest = json.loads((directory / "manifest.json").read_text(encoding="utf-8"))
        self.assertGreaterEqual(len(manifest["fixtures"]), 12)
        for case in manifest["fixtures"]:
            with self.subTest(case=case["path"]):
                self.assertEqual(
                    case.get("expected_reject_layer"),
                    "rust-python-semantic-or-decode",
                )
                with self.assertRaises(WireError) as caught:
                    decode_canonical(case["contract"], (directory / case["path"]).read_bytes())
                self.assertEqual(caught.exception.code, case["expected_error_code"])


if __name__ == "__main__":
    unittest.main()
