#!/usr/bin/env python3
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
INDEX = ROOT / "examples/golden-path/index.json"


def main() -> int:
    data = json.loads(INDEX.read_text(encoding="utf-8"))
    if data.get("schema_version") != "golden-path-index.v1":
        raise SystemExit("unsupported golden-path index")
    required = [
        data["capability_registry"],
        data["card_definition"],
        data["bundle_manifest"],
        *data["wire_fixtures"],
    ]
    missing = [rel for rel in required if not (ROOT / rel).is_file()]
    if missing:
        raise SystemExit(f"golden path references missing files: {missing}")

    subprocess.run(
        [sys.executable, "scripts/validate_schemas.py"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )

    census = subprocess.run(
        [
            sys.executable,
            "scripts/capability_census.py",
            "--bundle",
            data["bundle_manifest"],
            "--registry",
            data["capability_registry"],
            "--minimum-capability-lifecycle",
            "proposed",
            "--minimum-card-lifecycle",
            "draft",
            "--json",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if census.returncode != 0:
        raise SystemExit(
            f"golden-path census unexpectedly blocked:\n{census.stdout}\n{census.stderr}"
        )

    with tempfile.TemporaryDirectory() as temp:
        report = Path(temp) / "certification.json"
        certification = subprocess.run(
            [
                sys.executable,
                "scripts/certify_bundle.py",
                "--bundle",
                data["bundle_manifest"],
                "--registry",
                data["capability_registry"],
                "--output",
                str(report),
                "--engine-build",
                "foundation-0.2.2-golden-path",
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        if certification.returncode != 2:
            raise SystemExit(
                "golden-path certification must fail closed with exit 2, "
                f"got {certification.returncode}:\n"
                f"{certification.stdout}\n{certification.stderr}"
            )
        payload = json.loads(report.read_text(encoding="utf-8"))
        if payload.get("status") != data["expected_certification_status"]:
            raise SystemExit(f"unexpected certification status: {payload.get('status')}")

    print("PASS: synthetic golden path is structurally closed and certification fails closed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
