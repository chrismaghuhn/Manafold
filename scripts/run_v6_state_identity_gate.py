#!/usr/bin/env python3
"""S3.P0 current V5-state/V6-checkpoint-and-replay identity gate."""

from __future__ import annotations

import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]

REQUIRED: tuple[tuple[str, str], ...] = (
    ("crates/mtgml-model/src/lib.rs", 'raw_digest!(FullStateDigestV5, "mtgml.full-state-digest.v5")'),
    ("crates/mtgml-model/src/lib.rs", 'raw_digest!(CheckpointDigestV6, "mtgml.checkpoint-digest.v6")'),
    ("crates/mtgml-state/src/engine.rs", "Result<FullStateDigestV5, StateDigestError>"),
    ("crates/mtgml-state/src/digest_v5.rs", '"full-state-digest-input.v5"'),
    ("crates/mtgml-state/src/digest_v5.rs", '"magic_sba_graveyard_order_v1"'),
    ("crates/mtgml-state/src/delta.rs", "pub before_digest: FullStateDigestV5"),
    ("crates/mtgml-persistence/src/checkpoint_digest.rs", "calculate_checkpoint_digest_v6"),
    ("crates/mtgml-persistence/src/checkpoint_digest.rs", '"environment-checkpoint-digest-input.v6"'),
    ("crates/mtgml-environment/src/checkpoint.rs", "pub struct EnvironmentCheckpointV6"),
    ("crates/mtgml-environment/src/controller.rs", "Result<EnvironmentCheckpointV6, ControllerError>"),
    ("crates/mtgml-environment/src/synthetic.rs", "ReplayRecorderV6"),
    ("crates/mtgml-environment/src/reference.rs", "ReplayRecorderV6"),
    ("crates/mtgml-replay/src/lib.rs", "AuthoritativeReplayV6"),
    ("crates/mtgml-replay/src/v6.rs", '"replay-manifest.v6"'),
    ("crates/mtgml-replay/src/v6.rs", '"replay-step.v6"'),
    ("crates/mtgml-replay/src/v6.rs", '"authoritative-replay.v6"'),
    ("crates/mtgml-replay/src/v6.rs", "pub response: DecisionResponseV2"),
    ("crates/mtgml-wire/src/fixtures.rs", '"authoritative-replay.v6"'),
    ("python/src/mtgml/persistence.py", "def calculate_checkpoint_digest_v6("),
    ("python/src/mtgml/_replay_v6.py", "class AuthoritativeReplayV6:"),
    ("python/src/mtgml/wire.py", '"replay-manifest.v6"'),
    ("schemas/replay-manifest.v6.schema.json", '"const": "replay-manifest.v6"'),
    ("schemas/authoritative-replay.v6.schema.json", '"const": "authoritative-replay.v6"'),
    ("wire/golden/replay-manifest.v6.json", '"schema_version":"replay-manifest.v6"'),
    ("wire/golden/authoritative-replay-empty.v6.json", '"schema_version":"authoritative-replay.v6"'),
)

CURRENT_NO_OLD_IDS: tuple[str, ...] = (
    "crates/mtgml-state/src/engine.rs",
    "crates/mtgml-state/src/delta.rs",
    "crates/mtgml-environment/src/controller.rs",
    "crates/mtgml-environment/src/synthetic.rs",
    "crates/mtgml-environment/src/synthetic/commit.rs",
    "crates/mtgml-environment/src/synthetic/replay.rs",
    "crates/mtgml-environment/src/reference.rs",
    "crates/mtgml-environment/src/replay.rs",
    "crates/mtgml-conformance/src/facade.rs",
    "crates/mtgml-conformance/src/isolation/fingerprint.rs",
    "tools/m2-semantic-adapter/src/config.rs",
)

FORBIDDEN_CURRENT_TOKENS = (
    "FullStateDigestV4",
    "EnvironmentCheckpointV5",
    "CheckpointDigestV5",
    "ReplayManifestV5",
    "ReplayStepV5",
    "AuthoritativeReplayV5",
    "ReplayRecorderV5",
    "InitialEnvironmentIdentityV5",
)


def main() -> None:
    failures: list[str] = []
    for relative, token in REQUIRED:
        path = ROOT / relative
        if not path.is_file():
            failures.append(f"missing required V6 surface file: {relative}")
            continue
        if token not in path.read_text(encoding="utf-8"):
            failures.append(f"{relative}: missing current identity token {token!r}")

    for relative in CURRENT_NO_OLD_IDS:
        path = ROOT / relative
        if not path.is_file():
            failures.append(f"missing current consumer file: {relative}")
            continue
        content = path.read_text(encoding="utf-8")
        for token in FORBIDDEN_CURRENT_TOKENS:
            if token in content:
                failures.append(f"{relative}: current consumer still uses {token}")

    # Old identities remain available only in their explicitly versioned
    # historical verifier/type families and their frozen fixtures.
    historical_requirements = (
        ("crates/mtgml-state/src/digest_v4.rs", "calculate_full_state_digest_v4_historical"),
        ("crates/mtgml-environment/src/checkpoint.rs", "pub struct EnvironmentCheckpointV5"),
        ("crates/mtgml-replay/src/v5.rs", "pub struct ReplayManifestV5"),
        ("crates/mtgml-environment/tests/checkpoint_v5_red.rs", "new_builds_and_self_validates_a_v5_checkpoint"),
        ("crates/mtgml-replay/tests/replay_v5_red.rs", "replay_v5_v4_types_remain_untouched_and_pass"),
    )
    for relative, token in historical_requirements:
        path = ROOT / relative
        if not path.is_file() or token not in path.read_text(encoding="utf-8"):
            failures.append(f"historical V4/V5 exact verifier evidence missing: {relative} {token}")

    if failures:
        print(f"FAIL: {len(failures)} V6 state identity gate violation(s)")
        for failure in failures:
            print(f"  - {failure}")
        raise SystemExit(1)
    print("PASS: current FullStateDigestV5 / Checkpoint V6 / Replay V6 identity chain")
    print("PASS: current writers use V6; V4/V5 exact historical evidence remains present")


if __name__ == "__main__":
    main()
