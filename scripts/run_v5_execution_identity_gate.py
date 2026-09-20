#!/usr/bin/env python3
"""V5 Execution-Identity Gate (spec Task 12 / ADR 0055 §21/§22/§23a.1).

Asserts the V5 current-state identity surface and enforces the residual-V4
census (§22) plus the resolved kernel-construction census (§23a.1).

The gate is intentionally RED until the current producers/consumers from the
V4 census are migrated to V5 (Task 13).  Its RED state at this point IS the
required evidence.

Exit codes:
    0  — GREEN  (all V5-current tokens present, §23a.1 clean, no residual V4 producers/consumers)
    1  — RED    (one or more violations named in the report)
"""
from __future__ import annotations

import codecs
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

sys.dont_write_bytecode = True

# Ensure UTF-8 output on all platforms (Windows console may default to cp1252).
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="surrogateescape")
elif sys.stdout.encoding and sys.stdout.encoding.lower().startswith("cp"):
    sys.stdout = codecs.getwriter("utf-8")(sys.stdout.buffer, errors="replace")

ROOT = Path(__file__).resolve().parents[1]

# ---------------------------------------------------------------------------
# §21 V5-current tokens — these must be present in the current runtime surface.
# ---------------------------------------------------------------------------
# (relative_path, token, expected_minimum_occurrences_in_file_contents)
V5_CURRENT_TOKENS: list[tuple[str, str, int]] = [
    ("crates/mtgml-environment/src/checkpoint.rs", "EnvironmentCheckpointV5", 1),
    ("crates/mtgml-environment/src/checkpoint.rs", "ENVIRONMENT_CHECKPOINT_SCHEMA_V5", 1),
    ("crates/mtgml-environment/src/checkpoint.rs", "CHECKPOINT_CODEC_SEMANTIC_VERSION_V5", 1),
    ("crates/mtgml-model/src/lib.rs", "CheckpointDigestV5", 1),
    ("crates/mtgml-model/src/lib.rs", "ExecutionProgramV1", 1),
    ("crates/mtgml-model/src/lib.rs", "ExecutionIdentityV1", 1),
    ("crates/mtgml-model/src/lib.rs", "RulesContractIdV1", 1),
    ("crates/mtgml-model/src/lib.rs", "SemanticContractIdV1", 1),
    ("crates/mtgml-model/src/lib.rs", "FormatContractIdV1", 1),
    ("crates/mtgml-model/src/lib.rs", "ContentContractIdV1", 1),
    ("crates/mtgml-model/src/semantic_contract.rs", "SemanticContractManifestV1", 1),
    ("crates/mtgml-model/src/semantic_contract.rs", "RulesContractManifestV1", 1),
    ("crates/mtgml-model/src/semantic_contract.rs", "RulesAuthorityV1", 1),
    ("crates/mtgml-persistence/src/checkpoint_digest.rs", "calculate_checkpoint_digest_v5", 1),
    ("crates/mtgml-persistence/src/checkpoint_digest.rs", "CHECKPOINT_DOMAIN_V5", 1),
    ("crates/mtgml-environment/src/lib.rs", "EnvironmentCheckpointV5", 1),
    ("crates/mtgml-environment/src/lib.rs", "CHECKPOINT_CODEC_ID_V5", 1),
    ("crates/mtgml-replay/src/lib.rs", "ReplayManifestV5", 1),
    ("crates/mtgml-replay/src/lib.rs", "ReplayStepV5", 1),
    ("crates/mtgml-replay/src/lib.rs", "AuthoritativeReplayV5", 1),
    ("crates/mtgml-replay/src/lib.rs", "ReplayRecorderV5", 1),
    ("crates/mtgml-replay/src/lib.rs", "ReplaySchemaVersionsV5", 1),
    ("crates/mtgml-replay/src/lib.rs", "InitialEnvironmentIdentityV5", 1),
    ("crates/mtgml-replay/src/lib.rs", "SemanticContractMaterialV5", 1),
    ("crates/mtgml-replay/src/v5.rs", "replay-manifest.v5", 1),
    ("crates/mtgml-rules/src/program_kernel.rs", "ProgramKernelV1", 1),
    ("crates/mtgml-rules/src/program_kernel.rs", "ProgramKernelConstructionErrorV1", 1),
    ("crates/mtgml-rules/src/lib.rs", "ProgramKernelV1", 1),
    ("crates/mtgml-rules/src/program_kernel.rs", "for_program", 1),
    ("python/src/mtgml/persistence.py", "calculate_checkpoint_digest_v5", 1),
    ("python/src/mtgml/persistence.py", "CHECKPOINT_DOMAIN_V5", 1),
    ("python/src/mtgml/_replay_v5.py", "ReplayManifestV5", 1),
    ("python/src/mtgml/_replay_v5.py", "ReplayStepV5", 1),
    ("python/src/mtgml/_replay_v5.py", "AuthoritativeReplayV5", 1),
    ("python/src/mtgml/_replay_v5.py", "InitialEnvironmentIdentityV5", 1),
    ("python/src/mtgml/_replay_v5.py", "ReplaySchemaVersionsV5", 1),
    ("python/src/mtgml/_replay_v5.py", "SemanticContractMaterialV5", 1),
    ("python/src/mtgml/replay.py", "ReplayManifestV5", 1),
    ("python/src/mtgml/wire.py", "replay-manifest.v5", 1),
    ("python/src/mtgml/__init__.py", "calculate_checkpoint_digest_v5", 1),
    ("schemas/replay-manifest.v5.schema.json", "replay-manifest.v5", 1),
    ("schemas/authoritative-replay.v5.schema.json", "authoritative-replay.v5", 1),
    ("contracts/catalog/semantic-contracts.v1.json", "semantic-contracts-catalog.v1", 1),
    ("scripts/generate_semantic_contract_catalog.py", "generate_semantic_contract_catalog", 1),
    ("crates/mtgml-environment/src/semantic_catalog_generated.rs", "SemanticContractIdV1", 1),
]

# ---------------------------------------------------------------------------
# §23a.1 SyntheticM1RulesKernel site classifier.
# ---------------------------------------------------------------------------
class KernelSiteClass:
    ALLOWED_DECLARATION = "ALLOWED: declaration (struct/enum definition)"
    ALLOWED_IMPL = "ALLOWED: impl block"
    ALLOWED_CHILD_IMPL = "ALLOWED: internal child-module impl reference"
    ALLOWED_IMPORT = "ALLOWED: internal import to boundary/declaration module"
    ALLOWED_CONSTRUCTION = "ALLOWED: single ProgramKernelInner construction"
    ALLOWED_DOC = "ALLOWED: doc comment reference"
    FORBIDDEN_REEXPORT = "FORBIDDEN: public re-export"
    FORBIDDEN_EXTERNAL_IMPORT = "FORBIDDEN: external import (use mtgml_rules::…)"
    FORBIDDEN_LITERAL = "FORBIDDEN: direct literal outside allowed scope"
    FORBIDDEN_TEST_LITERAL = "FORBIDDEN: direct literal in test"
    FORBIDDEN_TOOL_LITERAL = "FORBIDDEN: direct literal in tool"

@dataclass
class KernelFinding:
    path: str
    line: int
    text: str
    classification: str
    disposition: str  # "ALLOWED" or "FORBIDDEN"

@dataclass
class KernelResult:
    findings: list[KernelFinding] = field(default_factory=list)

    @property
    def violations(self) -> list[KernelFinding]:
        return [f for f in self.findings if f.disposition == "FORBIDDEN"]

# Files and line-pattern maps for §23a.1 ALLOWED classification.
# The classifier matches per-line/per-site, never per-file.
KERNEL_ALLOWED_SITES: dict[str, list[tuple[str, str]]] = {
    # (line_pattern_regex, classification)
    "crates/mtgml-rules/src/synthetic.rs": [
        (r"^\s*pub struct SyntheticM1RulesKernel\b", KernelSiteClass.ALLOWED_DECLARATION),
        (r"^\s*impl\s+(RulesKernel for\s+)?SyntheticM1RulesKernel\b", KernelSiteClass.ALLOWED_IMPL),
    ],
    "crates/mtgml-rules/src/synthetic/stages.rs": [
        (r"^\s*use\s+super::SyntheticM1RulesKernel\b", KernelSiteClass.ALLOWED_CHILD_IMPL),
        (r"^\s*impl\s+SyntheticM1RulesKernel\b", KernelSiteClass.ALLOWED_CHILD_IMPL),
    ],
    "crates/mtgml-rules/src/program_kernel.rs": [
        (r"^\s*use\s+crate::synthetic::\{[^}]*SyntheticM1RulesKernel[^}]*\}", KernelSiteClass.ALLOWED_IMPORT),
        (r"^\s*use\s+crate::synthetic::.*SyntheticM1RulesKernel", KernelSiteClass.ALLOWED_IMPORT),
        (r"^\s*///", KernelSiteClass.ALLOWED_DOC),  # doc comment references
        (r"^\s*//.*", KernelSiteClass.ALLOWED_DOC),  # regular comment references
        (r"^\s*SyntheticLegacy\(SyntheticM1RulesKernel\)", KernelSiteClass.ALLOWED_CONSTRUCTION),
        (r"^\s*synthetic.*SyntheticM1RulesKernel", KernelSiteClass.ALLOWED_IMPORT),
    ],
    "crates/mtgml-rules/src/lib.rs": [
        # No SyntheticM1RulesKernel re-export allowed here; if it appears, it's FORBIDDEN_REEXPORT
        # (but validate_synthetic_runtime_state re-export is fine — we only flag SyntheticM1RulesKernel)
    ],
}

# Files where any SyntheticM1RulesKernel reference is FORBIDDEN.
KERNEL_FORBIDDEN_FILES: set[str] = {
    "crates/mtgml-rules/src/lib.rs",  # any SyntheticM1RulesKernel re-export is forbidden
}

def classify_kernel_site(rel_path: str, line: int, text: str) -> tuple[str, str] | None:
    """Return (classification, disposition) or None if not a kernel site."""

    norm_path = rel_path.replace("\\", "/")
    match norm_path:
        case p if p in KERNEL_ALLOWED_SITES:
            # Check allowed patterns
            for pattern, classification in KERNEL_ALLOWED_SITES[p]:
                if re.search(pattern, text):
                    return classification, "ALLOWED"
            # If in an allowed file but didn't match an allowed pattern AND
            # the line contains SyntheticM1RulesKernel, it's a direct literal
            # that slipped through.
            if "SyntheticM1RulesKernel" in text:
                # Check if it's a doc/comment line
                if re.match(r"^\s*(//|///|//!)", text):
                    # Comments in allowed files are allowed context
                    return KernelSiteClass.ALLOWED_DOC, "ALLOWED"
                # Any non-matched line with SyntheticM1RulesKernel in an allowed
                # file is a direct literal that doesn't match an allowed pattern.
                # In synthetic.rs this includes the struct field type reference,
                # use statements within the module, etc. — classify based on context.
                if p == "crates/mtgml-rules/src/synthetic.rs":
                    # The struct is module-private; references within the file
                    # to the type itself (field types, etc.) are internal.
                    if re.search(r"^\s*(pub\s+)?struct\s+", text):
                        return KernelSiteClass.ALLOWED_DECLARATION, "ALLOWED"
                    if re.search(r"^\s*(pub\s+)?use\s+", text):
                        return KernelSiteClass.ALLOWED_IMPORT, "ALLOWED"
                    return KernelSiteClass.ALLOWED_IMPL, "ALLOWED"

            # Comments without the token are not kernel sites
            return None

        case p if p in KERNEL_FORBIDDEN_FILES:
            if "SyntheticM1RulesKernel" in text:
                if re.search(r"^\s*pub\s+use\s+.*SyntheticM1RulesKernel", text):
                    return KernelSiteClass.FORBIDDEN_REEXPORT, "FORBIDDEN"
                if re.search(r"^\s*use\s+.*SyntheticM1RulesKernel", text):
                    return KernelSiteClass.FORBIDDEN_EXTERNAL_IMPORT, "FORBIDDEN"
                if "SyntheticM1RulesKernel" in text:
                    return KernelSiteClass.FORBIDDEN_LITERAL, "FORBIDDEN"
            return None

        case "crates/mtgml-rules/src/synthetic/runtime.rs":
            # Internal child module of synthetic/ - allowed impl references
            if "SyntheticM1RulesKernel" in text:
                if re.match(r"^\s*(//|///|//!)", text):
                    return KernelSiteClass.ALLOWED_DOC, "ALLOWED"
                return KernelSiteClass.ALLOWED_CHILD_IMPL, "ALLOWED"
            return None

        case "crates/mtgml-rules/src/synthetic/helpers.rs":
            # Internal child module of synthetic/ - allowed impl references
            if "SyntheticM1RulesKernel" in text:
                if re.match(r"^\s*(//|///|//!)", text):
                    return KernelSiteClass.ALLOWED_DOC, "ALLOWED"
                return KernelSiteClass.ALLOWED_CHILD_IMPL, "ALLOWED"
            return None

        case _:
            # Any other file: FORBIDDEN
            if "SyntheticM1RulesKernel" in text:
                stripped = text.strip()
                # Skip comment-only lines in non-allowed files (they're documentation,
                # not constructions/imports) — but still flag as informational
                if re.match(r"^\s*(//|///|//!)", text):
                    return KernelSiteClass.ALLOWED_DOC, "ALLOWED"
                # Determine context for classification
                is_test = "tests/" in norm_path or "/tests/" in norm_path or norm_path.endswith("_test.rs") or norm_path.endswith("_red.rs")
                is_tool = norm_path.startswith("tools/")
                if re.search(r"^\s*use\s+(mtgml_rules|crate::rules).*SyntheticM1RulesKernel", text) or \
                   re.search(r"^\s*use\s+.*\{.*SyntheticM1RulesKernel.*\}", text):
                    return KernelSiteClass.FORBIDDEN_EXTERNAL_IMPORT, "FORBIDDEN"
                if is_test:
                    return KernelSiteClass.FORBIDDEN_TEST_LITERAL, "FORBIDDEN"
                if is_tool:
                    return KernelSiteClass.FORBIDDEN_TOOL_LITERAL, "FORBIDDEN"
                return KernelSiteClass.FORBIDDEN_LITERAL, "FORBIDDEN"
            return None

def scan_kernel_sites() -> KernelResult:
    result = KernelResult()
    scan_dirs = [
        ROOT / "crates" / "mtgml-rules",
        ROOT / "crates" / "mtgml-environment",
        ROOT / "crates" / "mtgml-conformance",
        ROOT / "crates" / "mtgml-replay",
        ROOT / "crates" / "mtgml-model",
        ROOT / "crates" / "mtgml-persistence",
        ROOT / "crates" / "mtgml-state",
        ROOT / "crates" / "mtgml-wire",
        ROOT / "tools",
    ]
    for src_dir in scan_dirs:
        if not src_dir.exists():
            continue
        for path in sorted(src_dir.rglob("*.rs")):
            rel_path = str(path.relative_to(ROOT)).replace("\\", "/")
            text = path.read_text(encoding="utf-8")
            for lineno, line in enumerate(text.splitlines(), start=1):
                classification = classify_kernel_site(rel_path, lineno, line)
                if classification is not None:
                    classification_str, disposition = classification
                    result.findings.append(KernelFinding(
                        path=rel_path,
                        line=lineno,
                        text=line.rstrip()[:120],
                        classification=classification_str,
                        disposition=disposition,
                    ))
    return result


# ---------------------------------------------------------------------------
# §22 Residual-V4 census.
# ---------------------------------------------------------------------------
# V4 migration tokens: V4 types/identifiers that have V5 successors and must
# be migrated where they appear as CURRENT_PRODUCER/CURRENT_CONSUMER.
#
# NOT included: FullStateDigestV4 (still EXECUTABLE/current per ADR §2.12 —
# it has NO V5 successor), CheckpointDigestV3 and all V3 replay types (historical
# READABLE_VERIFIABLE_ONLY with no V5 successor), ReplaySchemaVersionsV1/V3/V4
# (only when in HISTORICAL_VERIFIER files).
#
# Per ADR §2.12: CheckpointDigestV4 / ReplayManifestV4 / ReplayStepV4 /
# AuthoritativeReplayV4 are READABLE_VERIFIABLE_ONLY (no V4 writer), but their
# Python *re-export* surface and conformance harness usage are CURRENT_CONSUMER
# — the RETAIN sites are the type definitions and historical digest functions.
V4_MIGRATION_TOKENS: tuple[str, ...] = (
    "EnvironmentCheckpointV4",
    "CheckpointDigestV4",
    "ReplayManifestV4",
    "ReplayStepV4",
    "AuthoritativeReplayV4",
    "ReplayRecorderV4",
    "InitialEnvironmentIdentityV4",
    "ReplaySchemaVersionsV4",
    "calculate_checkpoint_digest_v4",
    "replay-manifest.v4",
    "authoritative-replay.v4",
    "replay-step.v4",
    "environment-checkpoint.v4",
    "environment-checkpoint-digest-input.v4",
    "REPLAY_FILE_SCHEMA_V4",
    "REPLAY_MANIFEST_SCHEMA_V4",
    "REPLAY_STEP_SCHEMA_V4",
    "CHECKPOINT_CODEC_ID_V4",
    "CHECKPOINT_CODEC_SEMANTIC_VERSION_V4",
)

# RETAIN rows: V4 tokens permitted here (HISTORICAL_VERIFIER / FROZEN_FIXTURE /
# DOC_HISTORY disposition per §22). Each entry: (rel_path_prefix, token_subset_or_empty).
# Empty tuple means ALL V4 migration tokens are retained at this site.
V4_RETAIN_RULES: list[tuple[str, tuple[str, ...]]] = [
    # Checkpoint V4 retained for historical validation only (§17 writer posture)
    ("crates/mtgml-environment/src/checkpoint.rs", ()),
    # Integration test files (crate-root tests/) using V4 for RED/historical evidence
    ("crates/mtgml-environment/tests/", ()),
    ("crates/mtgml-rules/tests/p0_red.rs", ()),
    ("crates/mtgml-rules/tests/", ("ReplayStepV4",
        "EnvironmentCheckpointV4", "ReplayRecorderV4", "ReplayManifestV4",
        "ReplaySchemaVersionsV4", "InitialEnvironmentIdentityV4",
        "AuthoritativeReplayV4", "ReplayStepV4", "CheckpointDigestV4",
        "calculate_checkpoint_digest_v4", "REPLAY_FILE_SCHEMA_V4",
        "REPLAY_MANIFEST_SCHEMA_V4", "REPLAY_STEP_SCHEMA_V4",
        "CHECKPOINT_CODEC_ID_V4", "CHECKPOINT_CODEC_SEMANTIC_VERSION_V4",
        "replay-manifest.v4", "authoritative-replay.v4", "replay-step.v4",
        "environment-checkpoint.v4", "environment-checkpoint-digest-input.v4")),
    # V4 replay types retained as historical (READABLE_VERIFIABLE_ONLY)
    ("crates/mtgml-replay/src/v4.rs", ()),
    ("crates/mtgml-replay/src/v3.rs", ()),
    ("crates/mtgml-replay/src/v2.rs", ()),
    ("crates/mtgml-replay/src/v1.rs", ()),
    ("crates/mtgml-replay/src/identity.rs", ()),  # ReplaySchemaVersionsV4 + V1 historical
    # Wire dispatch for V4 fixtures (HISTORICAL_VERIFIER — V4 decoder retained)
    ("crates/mtgml-wire/src/fixtures.rs", ()),
    ("crates/mtgml-wire/src/lib.rs", ()),
    # V4 digest newtype retained in model (CheckpointDigestV4 is historical verifier)
    ("crates/mtgml-model/src/lib.rs", ()),
    # V4 persistence functions retained for historical digest recompute
    ("crates/mtgml-persistence/src/checkpoint_digest.rs", ()),
    # Python V4 persistence/replay retained as historical
    ("python/src/mtgml/persistence.py", ()),
    ("python/src/mtgml/_replay_v4.py", ()),
    # _replay_v5.py imports shared V4-named types (EnvironmentLimitCountersV4,
    # CheckpointCodecIdentityV4) that are NOT versioned — they carry the same
    # identity in V4 and V5. Only these tokens are retained here; any other
    # V4 migration token is a violation.
    ("python/src/mtgml/_replay_v5.py", (
        "EnvironmentLimitCountersV4", "CheckpointCodecIdentityV4",
        "CHECKPOINT_CODEC_ID_V4", "CHECKPOINT_CODEC_VERSION_V4",
    )),
    # Schema inventory lists V4 schema filenames (DOC_HISTORY)
    ("schemas/README.json", ()),
    ("schemas/replay-manifest.v4.schema.json", ()),
    ("schemas/authoritative-replay.v4.schema.json", ()),
    # Wire fixtures
    ("wire/golden/", ()),
    ("wire/negative/", ()),
    # P0 frozen fixtures — integration tests at crate root
    ("crates/mtgml-environment/tests/", ()),
    ("crates/mtgml-replay/tests/", ()),
    ("crates/mtgml-model/tests/", ()),
    ("crates/mtgml-persistence/tests/", ()),
    ("crates/mtgml-rules/tests/", ()),
    ("crates/mtgml-conformance/tests/", ()),
    ("python/tests/test_p0_red.py", ()),
    ("python/tests/test_m3_p0_green03.py", ()),
    # Historical scripts (DOC_HISTORY / HISTORICAL_VERIFIER)
    ("scripts/run_m1_closure.py", ()),
    ("scripts/run_m2_final_closure.py", ()),
    ("scripts/run_m2_h_gates.py", ()),
    ("scripts/validate_schemas.py", ()),
    # The gate script itself contains V4 vocabulary as detection patterns
    ("scripts/run_v5_execution_identity_gate.py", ()),
    # README and roadmap
    ("README.md", ()),
    ("docs/ROADMAP.md", ()),
    ("docs/README.md", ()),
    # Documentation files referencing V4 as historical context (DOC_HISTORY)
    ("docs/", ()),
]

# CURRENT_PRODUCER sites: V4 tokens here are VIOLATIONS.
# These are the production checkpoint/replay construction paths that must
# migrate to V5 per §22 census.
V4_CURRENT_PRODUCER_SITES: set[str] = {
    "crates/mtgml-environment/src/synthetic.rs",
    "crates/mtgml-environment/src/synthetic/commit.rs",
    "crates/mtgml-environment/src/synthetic/replay.rs",
    "crates/mtgml-environment/src/replay.rs",
    "crates/mtgml-environment/src/replay_parity_tests.rs",
    # Conformance harness (spec §22: CURRENT_CONSUMER → MUST MIGRATE)
    "crates/mtgml-conformance/src/facade.rs",
    "crates/mtgml-conformance/src/isolation/paired.rs",
    "crates/mtgml-conformance/src/isolation/replay_parity.rs",
    "crates/mtgml-conformance/src/isolation/checkpoint_parity.rs",
    "crates/mtgml-conformance/src/isolation/fork_parity.rs",
    "crates/mtgml-conformance/src/isolation/rejection.rs",
    "crates/mtgml-conformance/src/isolation/fingerprint.rs",
    "crates/mtgml-conformance/src/isolation/endpoint_pair.rs",
    "crates/mtgml-conformance/src/legal_space/gate_evidence.rs",
}

# STALE rows: V4 tokens in scripts whose V4-current blocks must be removed
# in Task 14.  These are NOT on RETAIN rules — they are flagged so Task 14
# knows to remove them.  (verify_repository.py's V4-current-block and
# run_m2_b_contract_cut.py's current-successor block are STALE per §22.)
V4_STALE_FILES: dict[str, str] = {
    "scripts/verify_repository.py": "STALE -> REMOVE (V4-current block replaced by V5 gate, Task 14)",
    "scripts/run_m2_b_contract_cut.py": "STALE -> REMOVE (current-successor block moves to V5 gate, Task 14)",
}

# CURRENT_CONSUMER sites: V4 re-exports / V4 imports that are current consumer surface.
# These MUST MIGRATE TO V5 per §22.
V4_CURRENT_CONSUMER_SITES: set[str] = {
    "crates/mtgml-environment/src/lib.rs",  # V4 re-exports in production non-test surface
    "crates/mtgml-environment/src/controller.rs",  # V4 trait + TrustedEnvironmentController signatures
    "crates/mtgml-replay/src/lib.rs",  # V4 pub use re-exports
    # Internal test module root + test subdirectory (spec §22: "tests.rs, tests/")
    "crates/mtgml-environment/src/tests.rs",
    # Files under src/tests/ that are NOT already in CURRENT_PRODUCER_SITES
    # are CURRENT_CONSUMER — catch them by path prefix.
    "python/src/mtgml/replay.py",
    "python/src/mtgml/wire.py",
    "python/src/mtgml/__init__.py",
    "crates/mtgml-wire/src/replay.rs",
    "tools/m2-semantic-adapter/src/config.rs",
    "tools/m2-semantic-adapter/src/session.rs",
}

@dataclass
class V4Finding:
    path: str
    line: int
    text: str
    token: str
    disposition: str  # CURRENT_PRODUCER, CURRENT_CONSUMER, HISTORICAL_VERIFIER, FROZEN_FIXTURE, etc.

@dataclass
class V4Result:
    violations: list[V4Finding] = field(default_factory=list)

def _path_matches_rules(rel_path: str) -> tuple[str, ...] | None:
    """Check if this path matches any RETAIN rule. Returns the token tuple (empty = all)."""
    norm = rel_path.replace("\\", "/")
    for rule_path, tokens in V4_RETAIN_RULES:
        rule_norm = rule_path.replace("\\", "/")
        if norm.startswith(rule_norm):
            return tokens
    return None

def _is_comment_line(line: str) -> bool:
    """True if the line is purely a comment (not code containing a comment)."""
    stripped = line.strip()
    return stripped.startswith("//") or stripped.startswith("#")


def census_v4() -> V4Result:
    result = V4Result()
    scan_dirs = [ROOT / "crates", ROOT / "tools", ROOT / "python" / "src", ROOT / "schemas", ROOT / "wire", ROOT / "scripts", ROOT / "docs"]
    for src_dir in scan_dirs:
        if not src_dir.exists():
            continue
        for path in sorted(src_dir.rglob("*")):
            if path.is_dir():
                continue
            ext = path.suffix
            if ext not in {".rs", ".py", ".json", ".md", ".toml", ".yml", ".yaml"}:
                continue
            rel_path = str(path.relative_to(ROOT)).replace("\\", "/")
            try:
                text = path.read_text(encoding="utf-8")
            except (UnicodeDecodeError, OSError):
                continue
            for lineno, line in enumerate(text.splitlines(), start=1):
                if _is_comment_line(line):
                    continue
                for token in V4_MIGRATION_TOKENS:
                    if token in line:
                        retain_tokens = _path_matches_rules(rel_path)
                        if retain_tokens is not None:
                            if retain_tokens == ():
                                continue
                            elif token in retain_tokens:
                                continue
                            else:
                                result.violations.append(V4Finding(
                                    path=rel_path, line=lineno, text=line.rstrip()[:120], token=token,
                                    disposition="HISTORICAL_VERIFIER (token not in retain set)",
                                ))
                                continue
                        elif rel_path in V4_STALE_FILES:
                            result.violations.append(V4Finding(
                                path=rel_path, line=lineno, text=line.rstrip()[:120], token=token,
                                disposition=V4_STALE_FILES[rel_path],
                            ))
                        elif rel_path in V4_CURRENT_PRODUCER_SITES:
                            result.violations.append(V4Finding(
                                path=rel_path, line=lineno, text=line.rstrip()[:120], token=token,
                                disposition="CURRENT_PRODUCER -> MUST MIGRATE TO V5",
                            ))
                        elif rel_path in V4_CURRENT_CONSUMER_SITES:
                            result.violations.append(V4Finding(
                                path=rel_path, line=lineno, text=line.rstrip()[:120], token=token,
                                disposition="CURRENT_CONSUMER -> MUST MIGRATE TO V5",
                            ))
                        elif rel_path.startswith("crates/mtgml-environment/src/tests/"):
                            result.violations.append(V4Finding(
                                path=rel_path, line=lineno, text=line.rstrip()[:120], token=token,
                                disposition="CURRENT_CONSUMER -> MUST MIGRATE TO V5 (src/tests/)",
                            ))
                        elif rel_path == "crates/mtgml-environment/src/tests.rs":
                            result.violations.append(V4Finding(
                                path=rel_path, line=lineno, text=line.rstrip()[:120], token=token,
                                disposition="CURRENT_CONSUMER -> MUST MIGRATE TO V5 (tests.rs)",
                            ))
                        else:
                            result.violations.append(V4Finding(
                                path=rel_path, line=lineno, text=line.rstrip()[:120], token=token,
                                disposition="UNKNOWN_SITE -> review required",
                            ))
    return result


def check_v5_current_tokens() -> list[str]:
    """Verify all V5-current tokens are present. Returns list of missing tokens."""
    missing: list[str] = []
    for rel_path, token, min_count in V5_CURRENT_TOKENS:
        full_path = ROOT / rel_path
        if not full_path.exists():
            missing.append(f"{rel_path}: file missing (token {token})")
            continue
        text = full_path.read_text(encoding="utf-8")
        count = text.count(token)
        if count < min_count:
            missing.append(f"{rel_path}: token '{token}' expected ≥{min_count}, found {count}")
    return missing


def main() -> None:
    sys.path.insert(0, str(ROOT / "python" / "src"))

    violations: list[str] = []

    # --- Layer 1: V5-current tokens ---
    missing_tokens = check_v5_current_tokens()
    if missing_tokens:
        violations.extend(missing_tokens)

    # --- Layer 2: §23a.1 SyntheticM1RulesKernel site classifier ---
    kernel_result = scan_kernel_sites()
    for finding in kernel_result.violations:
        violations.append(
            f"§23a.1 FORBIDDEN: {finding.path}:{finding.line} "
            f"'{finding.classification}': {finding.text}"
        )

    # --- Layer 3: §22 residual-V4 census ---
    v4_result = census_v4()
    for finding in v4_result.violations:
        violations.append(
            f"§22 VIOLATION: {finding.path}:{finding.line} "
            f"token '{finding.token}' — {finding.disposition}: {finding.text}"
        )

    if violations:
        print(f"RED: {len(violations)} V5 execution-identity violation(s):")
        for v in violations:
            print(f"  - {v}")
        sys.exit(1)

    print("GREEN: V5 execution-identity gate — all checks pass")
    print(f"  - V5 current tokens: {len(V5_CURRENT_TOKENS)} verified")
    print(f"  - §23a.1 kernel sites: {len(kernel_result.findings)} checked, 0 forbidden")
    print(f"  - §22 residual-V4 census: 0 current-producer/consumer violations")


if __name__ == "__main__":
    main()
