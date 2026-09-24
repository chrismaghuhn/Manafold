# Capability Registry

`registry.json` is the project registry. Nine Initial Foundation capabilities
remain specified-only. `rules/turn-structure@0.1.0` is covered for the bounded
M3.S1 scope; `covered` is not `certified`, and this does not claim card, deck,
format, Commander, broad Magic, or playability support. `registry.example.json`
demonstrates the schema without claiming support.

`rules/draw-card@0.1.0` has a Block 3 implementation candidate and executable
interaction evidence; its lifecycle remains `specified` pending exact-head
review. S2 remains `implemented / not covered`.

Use `python scripts/scaffold_capability.py <key> <title>` to add a proposal. Lifecycle and certification rules are defined in `docs/cards/CAPABILITY_MODEL.md`.
