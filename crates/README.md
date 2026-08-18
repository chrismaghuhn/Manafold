# Rust Workspace

Crates follow the dependency direction in `docs/ARCHITECTURE.md`. They contain
public contracts and small validation examples, not a playable engine. Path-only
dependencies keep the foundation auditable; external dependencies require policy
review and a regenerated lockfile.
