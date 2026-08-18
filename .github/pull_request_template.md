## Scope

Describe one coherent change and name the contract, capability, or maintenance
area it affects.

## Evidence

- [ ] Red test or conformance evidence existed before semantic implementation.
- [ ] Happy and illegal paths are covered.
- [ ] `python scripts/verify_repository.py` passes.
- [ ] Rust formatting, Clippy, and tests pass, or an explicit toolchain blocker is recorded.
- [ ] Python formatting, lint, typing, and tests pass.

## Contract review

- [ ] No hidden player choice was randomized, auto-selected, or moved to an adapter.
- [ ] Legal-action soundness and completeness impact was assessed.
- [ ] Hidden-information and identifier exposure were reviewed.
- [ ] Replay, schema, and deterministic-RNG compatibility were reviewed.
- [ ] Unsupported semantics still fail closed.
- [ ] Performance claims include a committed scenario and raw evidence.

## Compatibility

State whether the change is internal, additive, behavior-changing, schema
breaking, replay breaking, or bundle recertification-requiring. Link an ADR when
architecture or public meaning changes.
