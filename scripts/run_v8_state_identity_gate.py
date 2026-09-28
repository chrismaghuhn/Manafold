#!/usr/bin/env python3
"""Preserve V6/V7 historical identities and enforce the single G0 V8 cut."""

from __future__ import annotations

import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]

REQUIRED: tuple[tuple[str, str], ...] = (
    (
        "crates/mtgml-model/src/lib.rs",
        'raw_digest!(FullStateDigestV5, "mtgml.full-state-digest.v5")',
    ),
    (
        "crates/mtgml-model/src/lib.rs",
        'raw_digest!(CheckpointDigestV6, "mtgml.checkpoint-digest.v6")',
    ),
    ("crates/mtgml-state/src/engine.rs", "Result<FullStateDigestV5, StateDigestError>"),
    ("crates/mtgml-state/src/digest_v5.rs", '"full-state-digest-input.v5"'),
    ("crates/mtgml-state/src/digest_v5.rs", '"magic_sba_graveyard_order_v1"'),
    ("crates/mtgml-state/src/delta.rs", "pub before_digest: FullStateDigestV5"),
    ("crates/mtgml-persistence/src/checkpoint_digest.rs", "calculate_checkpoint_digest_v6"),
    (
        "crates/mtgml-persistence/src/checkpoint_digest.rs",
        '"environment-checkpoint-digest-input.v6"',
    ),
    ("crates/mtgml-environment/src/checkpoint.rs", "pub struct EnvironmentCheckpointV6"),
    ("crates/mtgml-environment/src/controller.rs", "pub use crate::controller_successor::*;"),
    ("crates/mtgml-environment/src/endpoint.rs", "pub use crate::endpoint_successor::*;"),
    (
        "crates/mtgml-environment/src/controller_successor.rs",
        "Result<EnvironmentCheckpointV8, ControllerError>",
    ),
    (
        "crates/mtgml-environment/src/basic_land_runtime_v8.rs",
        "pub struct BasicLandEnvironmentRuntimeV8",
    ),
    ("crates/mtgml-environment/src/checkpoint_v8.rs", "pub struct EnvironmentCheckpointV8"),
    ("crates/mtgml-replay/src/v8.rs", "pub struct AuthoritativeReplayV8"),
    ("crates/mtgml-decision/src/v4.rs", "pub struct AuthoritativeDecisionRequestV4"),
    ("crates/mtgml-decision/src/v3.rs", "pub struct DecisionResponseV3"),
    ("crates/mtgml-observation/src/player_step_v4.rs", "pub struct PlayerStepV4"),
    ("crates/mtgml-environment/src/checkpoint_v7.rs", "pub struct EnvironmentCheckpointV7"),
    ("crates/mtgml-environment/src/replay_v7_execution.rs", "execute_authoritative_replay_v7"),
    ("crates/mtgml-rules/src/transition.rs", "state: &mtgml_state::EngineStatePartsV2"),
    ("crates/mtgml-state/src/engine_state_parts_v2.rs", "pub struct EngineStatePartsV2"),
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
    (
        "wire/golden/authoritative-replay-empty.v6.json",
        '"schema_version":"authoritative-replay.v6"',
    ),
)

CURRENT_NO_OLD_IDS: tuple[str, ...] = (
    "crates/mtgml-environment/src/controller.rs",
    "crates/mtgml-environment/src/endpoint.rs",
    "crates/mtgml-environment/src/controller_successor.rs",
    "crates/mtgml-environment/src/endpoint_successor.rs",
    "crates/mtgml-environment/src/basic_land_runtime_v8.rs",
)

FORBIDDEN_CURRENT_TOKENS = (
    "FullStateDigestV6",
    "StateDeltaV2",
    "EnvironmentCheckpointV7",
    "CheckpointDigestV7",
    "ReplayManifestV7",
    "ReplayStepV7",
    "AuthoritativeReplayV7",
    "ReplayRecorderV7",
    "InitialEnvironmentIdentityV7",
    "AuthoritativeDecisionRequestV3",
    "PlayerDecisionRequestV3",
    "DecisionResponseV2",
    "PlayerStepV3",
    "EngineStatePartsV2",
    "ExecutionStateV3",
    "ObservedEventEnvelopeV3",
    "PlayerInformationStateV2",
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
        if relative == "crates/mtgml-environment/src/basic_land_runtime_v8.rs":
            content = content.split("#[cfg(test)]\nmod tests", maxsplit=1)[0]
        for token in FORBIDDEN_CURRENT_TOKENS:
            if token in content:
                failures.append(f"{relative}: current consumer still uses {token}")

    v8_rules_bridge = (ROOT / "crates/mtgml-rules/src/basic_land_v4.rs").read_text(encoding="utf-8")
    for token in (
        "execute_basic_land_response(",
        "StateDeltaV2::between(",
        "DecisionResponseV2",
        "AuthoritativeDecisionRequestV3",
        "request_v3_from_v4",
        "request_v4_from_v3",
    ):
        if token in v8_rules_bridge:
            failures.append(f"V8 Rules bridge still produces predecessor transition value {token}")

    environment_lib = (ROOT / "crates/mtgml-environment/src/lib.rs").read_text(encoding="utf-8")
    successor_cut_requirements = (
        '#[cfg(any(test, feature = "historical-conformance-runtime"))]\n'
        '#[path = "controller_predecessor.rs"]\nmod controller;',
        '#[cfg(not(any(test, feature = "historical-conformance-runtime")))]\nmod controller;',
        '#[cfg(any(test, feature = "historical-conformance-runtime"))]\n'
        '#[path = "endpoint_predecessor.rs"]\nmod endpoint;',
        '#[cfg(not(any(test, feature = "historical-conformance-runtime")))]\nmod endpoint;',
        '#[cfg(any(test, feature = "historical-conformance-runtime"))]\npub mod successor_runtime;',
        '#[cfg(not(any(test, feature = "historical-conformance-runtime")))]\n'
        "pub type CurrentPlayerStep = mtgml_observation::PlayerStepV4;",
        '#[cfg(not(any(test, feature = "historical-conformance-runtime")))]\n'
        "pub use basic_land_runtime_v8::{",
    )
    for required in successor_cut_requirements:
        if required not in environment_lib:
            failures.append("environment default authority does not select the V8/V4 G0 path")

    rules_lib = (ROOT / "crates/mtgml-rules/src/lib.rs").read_text(encoding="utf-8")
    historical_program_export = (
        '#[cfg(any(test, feature = "historical-runtime-testkit"))]\n'
        "pub use program_kernel::{ProgramKernelConstructionErrorV1, ProgramKernelV1};"
    )
    if historical_program_export not in rules_lib:
        failures.append(
            "the V2 ProgramKernel executable API is not isolated as historical/test-only"
        )

    restricted_structural_surfaces = (
        (
            "crates/mtgml-state/src/delta_v3.rs",
            "#[doc(hidden)]\n    pub fn between_structural_only",
        ),
        (
            "crates/mtgml-state/src/delta_v3.rs",
            "#[doc(hidden)]\n    pub fn apply_structural_only",
        ),
        (
            "crates/mtgml-state/src/digest_v7.rs",
            "#[doc(hidden)]\npub fn calculate_full_state_digest_v7_structural_only",
        ),
        (
            "crates/mtgml-environment/src/checkpoint_v8.rs",
            "    fn validate_structural_only(",
        ),
        (
            "crates/mtgml-environment/src/player_projection.rs",
            "pub(crate) fn project_successor_information_state_v3_structural_only",
        ),
        (
            "crates/mtgml-rules/src/events_v3.rs",
            "pub(crate) fn validate_event_delta_state_v3_structural_only",
        ),
    )
    for relative, token in restricted_structural_surfaces:
        if token not in (ROOT / relative).read_text(encoding="utf-8"):
            failures.append(f"structural-only helper is not explicitly restricted: {relative}")
    for relative, token in (
        (
            "crates/mtgml-environment/src/lib.rs",
            "project_successor_information_state_v3_structural_only",
        ),
        (
            "crates/mtgml-rules/src/lib.rs",
            "validate_event_delta_state_v3_structural_only",
        ),
    ):
        if token in (ROOT / relative).read_text(encoding="utf-8"):
            failures.append(f"structural-only helper leaks through a public re-export: {relative}")

    for relative in (
        "crates/mtgml-environment/src/controller.rs",
        "crates/mtgml-environment/src/endpoint.rs",
    ):
        content = (ROOT / relative).read_text(encoding="utf-8")
        for token in (
            "EnvironmentCheckpointV6",
            "AuthoritativeReplayV6",
            "EnvironmentCheckpointV7",
            "AuthoritativeReplayV7",
            "PlayerStepV3",
            "PlayerDecisionRequestV3",
            "DecisionResponseV2",
        ):
            if token in content:
                failures.append(f"{relative}: predecessor runtime API remains current via {token}")

    # Old identities remain available only in their explicitly versioned
    # historical verifier/type families and their frozen fixtures.
    historical_requirements = (
        ("crates/mtgml-state/src/digest_v4.rs", "calculate_full_state_digest_v4_historical"),
        ("crates/mtgml-environment/src/checkpoint.rs", "pub struct EnvironmentCheckpointV5"),
        ("crates/mtgml-replay/src/v5.rs", "pub struct ReplayManifestV5"),
        (
            "crates/mtgml-environment/tests/checkpoint_v5_red.rs",
            "new_builds_and_self_validates_a_v5_checkpoint",
        ),
        (
            "crates/mtgml-replay/tests/replay_v5_red.rs",
            "replay_v5_v4_types_remain_untouched_and_pass",
        ),
    )
    for relative, token in historical_requirements:
        path = ROOT / relative
        if not path.is_file() or token not in path.read_text(encoding="utf-8"):
            failures.append(f"historical V4/V5 exact verifier evidence missing: {relative} {token}")

    if failures:
        print(f"FAIL: {len(failures)} V8 state identity gate violation(s)")
        for failure in failures:
            print(f"  - {failure}")
        raise SystemExit(1)
    print("PASS: V6/V7 identity definitions, codecs and historical fixtures remain exact")
    print("PASS: default environment/controller/endpoint select the single V8/V4 G0 authority")
    print("PASS: V7 executable runtime adapters are isolated behind test/conformance cfg")


if __name__ == "__main__":
    main()
