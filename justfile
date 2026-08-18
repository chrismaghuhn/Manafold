set shell := ["bash", "-euo", "pipefail", "-c"]

default: check-fast

doctor:
    python scripts/doctor.py

bootstrap:
    python scripts/bootstrap.py

generate-contracts:
    python scripts/generate_contracts.py

check-generated:
    python scripts/generate_contracts.py --check

check-fast:
    python scripts/run_checks.py fast

check:
    python scripts/run_checks.py integration

check-all:
    python scripts/run_checks.py certification

release-candidate:
    python scripts/run_verification.py

format:
    cargo fmt --all
    python -m ruff format python scripts

format-check:
    cargo fmt --all -- --check
    python -m ruff format --check python scripts

lint:
    cargo check --workspace --all-targets --all-features --locked
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    python -m ruff check python scripts
    python -m mypy --config-file python/pyproject.toml

unit:
    cargo test --workspace --all-features --locked
    PYTHONDONTWRITEBYTECODE=1 python scripts/run_python_tests.py

contracts:
    python scripts/generate_contracts.py --check
    python scripts/verify_repository.py
    python scripts/check_rust_source_structure.py
    python scripts/check_documentation.py
    python scripts/validate_schemas.py
    python scripts/validate_maintainer_artifacts.py
    python scripts/validate_golden_path.py
    python scripts/verify_python_toolchain.py

archive-check:
    python scripts/verify_archive_reproducibility.py

verify: check-all

new-adr title:
    python scripts/new_adr.py "{{title}}"

scaffold-card id name:
    python scripts/scaffold_card.py "{{id}}" "{{name}}"

scaffold-capability key title:
    python scripts/scaffold_capability.py "{{key}}" "{{title}}"

census bundle:
    python scripts/capability_census.py --bundle "{{bundle}}"

certify-preflight bundle output:
    python scripts/certify_bundle.py --bundle "{{bundle}}" --output "{{output}}"

archive:
    python scripts/build_source_archive.py --output-dir dist

verification-report: release-candidate
