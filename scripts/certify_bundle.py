#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.dont_write_bytecode = True

from maintainer_common import (
    ROOT,
    MaintainerArtifactError,
    capability_census,
    certification_closure,
    certification_status,
    digest_json,
    load_json,
    write_json,
)

REQUIRED_GATES = [
    "REFERENCE_IMPLEMENTATION",
    "EXACT_CONFORMANCE",
    "LEGAL_ACTION_SOUNDNESS",
    "LEGAL_ACTION_COMPLETENESS",
    "INFORMATION_NONINTERFERENCE",
    "REPLAY_CHECKPOINT_PARITY",
    "PROPERTY_FUZZ_SOAK",
    "PERFORMANCE_BUDGET",
    "CLEAN_MACHINE_REPRODUCTION",
]


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate a conservative static bundle-certification preflight.")
    parser.add_argument("--bundle", required=True, type=Path)
    parser.add_argument("--registry", type=Path)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--engine-build", default="UNVERIFIED")
    parser.add_argument("--backend", default="reference")
    args = parser.parse_args()

    try:
        bundle = load_json(args.bundle)
        registry = load_json(
            args.registry or args.root / "cards" / "capabilities" / "registry.json"
        )
        census = capability_census(
            bundle,
            registry,
            root=args.root,
            required_capability_lifecycle="certified",
            required_card_lifecycle="covered",
        )
    except MaintainerArtifactError as exc:
        parser.error(str(exc))
    gates = [{"gate": gate, "status": "NOT_RUN", "evidence": [], "reason": "No runtime evidence supplied to static preflight."} for gate in REQUIRED_GATES]
    status = certification_status(census, gates)
    closure_wire = certification_closure(census)
    report = {
        "schema_version": "bundle-certification.v1",
        "bundle_id": bundle["bundle_id"],
        "bundle_content_digest": bundle["content_digest"],
        "engine_build": args.engine_build,
        "backend": args.backend,
        "snapshots": bundle["snapshots"],
        "capability_closure": closure_wire,
        "gates": gates,
        "status": status,
        "exclusions": sorted(set(bundle.get("exclusions", []) + ["Static preflight cannot prove semantic correctness."])),
        "evidence_digest": "0" * 64,
        "generated_by": "scripts/certify_bundle.py@0.2.2",
    }
    report["evidence_digest"] = digest_json({k: v for k, v in report.items() if k != "evidence_digest"})
    write_json(args.output, report)
    print(f"{status}: {args.output}")
    raise SystemExit(0 if status == "certified" else 2)


if __name__ == "__main__":
    main()
