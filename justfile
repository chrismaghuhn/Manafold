set shell := ["bash", "-euo", "pipefail", "-c"]
project_python := ".venv/bin/python"

default: check-fast

doctor:
    python scripts/doctor.py

bootstrap:
    python scripts/bootstrap.py

generate-contracts:
    {{project_python}} scripts/generate_contracts.py

check-generated:
    {{project_python}} scripts/generate_contracts.py --check

check-fast:
    {{project_python}} scripts/run_checks.py fast

check:
    {{project_python}} scripts/run_checks.py integration

check-all:
    {{project_python}} scripts/run_checks.py certification

release-candidate:
    {{project_python}} scripts/run_verification.py

format:
    cargo fmt --all
    {{project_python}} -m ruff format python scripts

format-check:
    cargo fmt --all -- --check
    {{project_python}} -m ruff format --check python scripts

lint:
    cargo check --workspace --all-targets --all-features --locked
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    {{project_python}} -m ruff check python scripts
    {{project_python}} -m mypy --config-file python/pyproject.toml

unit:
    cargo test --workspace --all-features --locked
    PYTHONDONTWRITEBYTECODE=1 {{project_python}} scripts/run_python_tests.py

contracts:
    {{project_python}} scripts/generate_contracts.py --check
    {{project_python}} scripts/verify_repository.py
    {{project_python}} scripts/check_rust_source_structure.py
    {{project_python}} scripts/check_documentation.py
    {{project_python}} scripts/validate_schemas.py
    {{project_python}} scripts/validate_maintainer_artifacts.py
    {{project_python}} scripts/validate_golden_path.py
    {{project_python}} scripts/verify_python_toolchain.py

archive-check:
    {{project_python}} scripts/verify_archive_reproducibility.py

verify: check-all

new-adr title:
    {{project_python}} scripts/new_adr.py "{{title}}"

scaffold-card id name:
    {{project_python}} scripts/scaffold_card.py "{{id}}" "{{name}}"

scaffold-capability key title:
    {{project_python}} scripts/scaffold_capability.py "{{key}}" "{{title}}"

census bundle:
    {{project_python}} scripts/capability_census.py --bundle "{{bundle}}"

certify-preflight bundle output:
    {{project_python}} scripts/certify_bundle.py --bundle "{{bundle}}" --output "{{output}}"

archive:
    {{project_python}} scripts/build_source_archive.py --output-dir dist

verification-report: release-candidate
