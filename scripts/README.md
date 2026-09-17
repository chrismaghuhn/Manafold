# Maintainer Scripts

- `verify_repository.py` — structural repository and cross-layer contract checks;
- `check_rust_source_structure.py` — conservative Rust delimiter/comment/string balance check; never a substitute for Cargo;
- `check_documentation.py` — document register, ADR numbering, and local links;
- `validate_schemas.py` — JSON Schemas and examples;
- `validate_maintainer_artifacts.py` — capability, card, bundle, and certification semantics;
- `run_python_tests.py` — Python contract suite (`--profile full` is the default;
  `--profile smoke` is the explicit small development allowlist);
- `run_verification.py` — external authoritative gate report under `dist/verification/`;
- `run_m1_closure.py` — external M1 ten-gate closure report under `dist/verification/m1/`;
- `run_m2_final_closure.py` — external M2.Final twenty-gate closure report (M2.B-H runners plus M1 regression and the M2 scope guard) under `dist/m2-final-verification/`;
- `build_source_archive.py` — deterministic source ZIP and checksum;
- `verify_source_archive.py` — source/archive member and byte parity;
- `verify_archive_reproducibility.py` — repeated deterministic build and ZIP safety;
- `scaffold_card.py` / `scaffold_capability.py` — maintainer scaffolding;
- `capability_census.py` — recursive capability, definition, generated-object, and native-executor closure;
- `certify_bundle.py` — fail-closed static certification preflight.

Generated logs and status reports must not be written into the archived source set.

The verification runner marks directories it owns and refuses to replace an existing unmarked output directory.

- `generate_contracts.py` — single-source generation/check for mechanical Rust/Python/schema vocabulary;
- `run_checks.py` — fast (Smoke), integration (Smoke + Full), and certification
  maintainer profiles;
- `failure_packet.py` — internal trusted packet validation, source identities,
  checksums, atomic writes, and safe summaries;
- `capture_failure.py` — opt-in bounded command capture with `shell=False`;
  `CAPTURE_PASS` creates no packet, `CAPTURE_COMMAND_EXIT` preserves a
  nonzero exit in trusted evidence, `CAPTURE_TIMEOUT` records
  `COMMAND_TIMEOUT`, and unsafe or unavailable preconditions return
  `CAPTURE_BLOCKED`;
- `rerun_failure.py` — opt-in exact-head packet rerun with commit/tree/source
  fingerprint checks and `REPRODUCED`, `NOT_REPRODUCED`, or `BLOCKED` status;
- `run_dependency_audit.py` — explicit, bounded RustSec/PyPA dependency audits;
- `bootstrap.py` — prepares `.venv` only and never mutates contracts or lockfiles;
- `validate_golden_path.py` — verifies the synthetic vertical path fails closed at certification;

The failure-reproducer scripts are maintainer-only and are never invoked by
`run_checks.py`. The default generated packet location is
`dist/failures/`, owned by `.mtgml-failure-output`; an existing unowned root is
rejected. Packet logs are trusted local evidence, not public CI output,
player diagnostics, ML fields, replay/checkpoint authority, or semantic
fixtures. `command.argv` is the only executable argument list,
`command.cwd` is repository-relative, and any display rerun command is
informational only.

The explicit opt-in T0 failure witness (an ignored Rust test that runs a
real digest-mismatch T0 case, prints exactly one closed `T0_FAILURE_CONTEXT`
line plus exactly one `MANAFOLD_FAILURE_SIGNATURE`, and exits nonzero) is
captured and reproduced outside the source tree with:

```text
<project-python> scripts/capture_failure.py \
  --case-id synthetic-entry-digest-mismatch \
  --output-root <outside-source-or-dist/failures> \
  -- cargo +1.85.1 test --package mtgml-conformance --locked \
  t0_failure_witness_capture -- --ignored --nocapture

<project-python> scripts/rerun_failure.py <packet>
```

A declared `--case-id` that disagrees with the emitted T0 context is
blocked without a packet. Rerun requires exact T0 context equality
(case, step, diagnostic, authority, kernel, expected/actual digest
identities) in addition to the existing signature and exit checks.
