# Maintainer Scripts

- `verify_repository.py` — structural repository and cross-layer contract checks;
- `check_rust_source_structure.py` — conservative Rust delimiter/comment/string balance check; never a substitute for Cargo;
- `check_documentation.py` — document register, ADR numbering, and local links;
- `validate_schemas.py` — JSON Schemas and examples;
- `validate_maintainer_artifacts.py` — capability, card, bundle, and certification semantics;
- `run_python_tests.py` — Python contract suite;
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
- `authority_source_resolver.py` — rules-neutral, byte-first repository/REV3 source, locator, candidate, and source-instance resolution;
- `run_checks.py` — fast/integration/certification maintainer profiles;
- `bootstrap.py` — prepares `.venv` only and never mutates contracts or lockfiles;
- `validate_golden_path.py` — verifies the synthetic vertical path fails closed at certification;
